import { Stack, StackProps, Duration, RemovalPolicy, Tags } from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as sqs from 'aws-cdk-lib/aws-sqs';
import {
    aws_ec2 as ec2,
    aws_ecs as ecs,
    aws_iam as iam,
    aws_logs as logs,
} from 'aws-cdk-lib';

export class EcsSqsScalingStack extends Stack {
    constructor(scope: Construct, id: string, props?: StackProps) {
        super(scope, id, props);

        const queue = new sqs.Queue(this, 'EcsSqsScalingQueue', {
            visibilityTimeout: Duration.seconds(300),
            removalPolicy: RemovalPolicy.DESTROY,
        });

        // ECS Cluster
        const vpc = new ec2.Vpc(this, 'VPC', {});
        Tags.of(vpc).add('Name', 'SQSVPC');

        const cluster = new ecs.Cluster(this, 'Cluster', {
            vpc: vpc,
        });

        const ECSExecPolicyStatement = new iam.PolicyStatement({
            sid: 'allowECSExec',
            resources: ['*'],
            actions: [
                'ssmmessages:CreateControlChannel',
                'ssmmessages:CreateDataChannel',
                'ssmmessages:OpenControlChannel',
                'ssmmessages:OpenDataChannel',
                'logs:CreateLogStream',
                'logs:DescribeLogGroups',
                'logs:DescribeLogStreams',
                'logs:PutLogEvents',
            ],
        });

        const taskRole = new iam.Role(this, 'TaskRole', {
            assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
            managedPolicies: [
                {
                    managedPolicyArn:
                        'arn:aws:iam::aws:policy/AmazonSQSFullAccess',
                },
            ],
        });
        taskRole.addToPolicy(ECSExecPolicyStatement);

        const taskExecutionRole = new iam.Role(this, 'TaskExecutionRole', {
            assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
            managedPolicies: [
                {
                    managedPolicyArn:
                        'arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy',
                },
            ],
        });

        const logGroup = new logs.LogGroup(this, 'LogGroup', {
            logGroupName: 'cdk-sqs-scaling',
            removalPolicy: RemovalPolicy.DESTROY,
        });

        const taskDefinition = new ecs.FargateTaskDefinition(this, 'TaskDef', {
            memoryLimitMiB: 512,
            cpu: 256,
            executionRole: taskExecutionRole,
            taskRole: taskRole,
        });

        const image = new ecs.AssetImage('image');

        taskDefinition.addContainer('Queue-consumer', {
            image: image,
            environment: {
                SQS_URL: queue.queueUrl,
            },
            logging: ecs.LogDriver.awsLogs({
                streamPrefix: 'rust-queue-consumer',
                logGroup: logGroup,
            }),
        });

        const service = new ecs.FargateService(this, 'Service', {
            cluster: cluster,
            assignPublicIp: false,
            taskDefinition: taskDefinition,
            enableExecuteCommand: true,
        });

        const scaling = service.autoScaleTaskCount({
            maxCapacity: 5,
            minCapacity: 1,
        });

        scaling.scaleOnMetric('QueueDepthScaling', {
            metric: queue.metricApproximateNumberOfMessagesVisible(),
            scalingSteps: [
                {
                    change: 2,
                    lower: 4,
                },
                {
                    change: -2,
                    upper: 3,
                },
            ],
        });
    }
}
