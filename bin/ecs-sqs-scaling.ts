#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { EcsSqsScalingStack } from '../lib/ecs-sqs-scaling-stack';

const app = new cdk.App();
new EcsSqsScalingStack(app, 'EcsSqsScalingStack', {
    env: { region: 'us-east-1' },
});
