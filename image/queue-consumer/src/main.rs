use aws_sdk_sqs::{model::Message, Client, Config, Credentials, Error, Region};
use aws_smithy_client::erase::DynConnector;
use aws_smithy_client::test_connection::TestConnection;
use aws_smithy_http::body::SdkBody;
use ctrlc;
use http::Uri;
use std::{env, fs, process, thread, time};

fn get_sqs_url() -> String {
    let sqs_url = env::var("SQS_URL").expect("SQS_URL not set");
    sqs_url
}

#[tokio::main]
async fn get_messages(sqs_url: String, client: Client) -> Result<Vec<Message>, Error> {
    let received_message_output = client
        .receive_message()
        .queue_url(sqs_url)
        .max_number_of_messages(1)
        .send()
        .await?;
    let messages = received_message_output.messages.unwrap_or_default();
    Ok(messages)
}

#[tokio::main]
async fn delete_message(receipt_handle: String) -> Result<(), Error> {
    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);
    let sqs_url = get_sqs_url();
    let _delete_message_output = client
        .delete_message()
        .queue_url(sqs_url)
        .receipt_handle(receipt_handle)
        .send()
        .await?;
    Ok(())
}

fn sleep(sec: u64) {
    let sleep_secs = time::Duration::from_secs(sec);
    thread::sleep(sleep_secs);
    println!("{} seconds slept", sec);
}

fn extract_body(message: &Message) -> String {
    let body = message.body.as_ref().unwrap().to_string();
    body
}

fn process_message(message: &Message) {
    let body = extract_body(message);
    let receipt_handle = message.receipt_handle.as_ref().unwrap().to_string();
    println!("Message body is {}", body);
    sleep(250);
    let result = delete_message(receipt_handle);
    match result {
        Ok(_) => println!("Succeeded to delete message"),
        Err(e) => println!("Failed to delete message: {:?}", e),
    }
}

fn process_messages(messages: &Vec<Message>) {
    if messages.len() == 0 {
        println!("No message received")
    } else {
        process_message(&messages[0])
    }
}

// https://github.com/awslabs/aws-sdk-rust/issues/199#issuecomment-904558631
fn mock_client() -> Client {
    let creds = Credentials::new(
        "TESTCLIENT",
        "testsecretkey",
        Some("testsessiontoken".to_string()),
        None,
        "mock",
    );
    let config = Config::builder()
        .credentials_provider(creds)
        .region(Region::new("us-east-1"))
        .build();
    let data =
        fs::read_to_string("./data/receive_message.xml").expect("Failed to read mock response");
    let conn = TestConnection::new(vec![(
        http::Request::builder()
            .uri(Uri::from_static("https://sqs.us-east-1.amazonaws.com/"))
            .body(SdkBody::from(r#"{"NumberOfBytes":64}"#))
            .unwrap(),
        http::Response::builder()
            .status(http::StatusCode::from_u16(200).unwrap())
            .body(SdkBody::from(data))
            .unwrap(),
    )]);
    let conn = DynConnector::new(conn);
    let client = Client::from_conf_conn(config, conn);
    client
}

#[tokio::main]
async fn aws_client() -> Client {
    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);
    client
}

fn main() {
    // Handling SIGTERM
    ctrlc::set_handler(move || {
        println!("Received SIGTERM");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
    loop {
        let sqs_url = get_sqs_url();
        let client = aws_client();
        let messages = get_messages(sqs_url, client);
        match messages {
            Ok(v) => process_messages(&v),
            Err(e) => println!("Failed to get messages: {:?}", e),
        }
        sleep(10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_sqs_url() {
        env::set_var("SQS_URL", "TestSqsUrl");
        let sqs_url = get_sqs_url();
        assert_eq!(sqs_url, "TestSqsUrl".to_string());
    }
    #[test]
    fn test_receive_message() {
        env::set_var("SQS_URL", "TestSqsUrl");
        let sqs_url = get_sqs_url();
        let client = mock_client();
        let messages = get_messages(sqs_url, client);
        let body = extract_body(&messages.unwrap()[0]);
        assert_eq!(body, "This is a test message".to_string());
    }
}
