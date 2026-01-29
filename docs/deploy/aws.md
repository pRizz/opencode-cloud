# AWS Quick Deploy (One-Click)

Deploy opencode-cloud on AWS with a public Application Load Balancer (ALB) and
HTTPS via ACM, while keeping the EC2 instance private by default.

## Prerequisites

- AWS account with permissions to create EC2, ALB, ACM, and IAM resources.
- A domain name you control (required for ACM TLS validation).
- Ability to edit DNS records for the domain.
- A Route53 hosted zone for the domain (required for automated validation).

## Quick Deploy

1. Click the AWS deploy button in the root `README.md`.
2. Provide a **domain name** (e.g., `opencode.example.com`).
3. Provide the Route53 hosted zone ID for automatic DNS validation.
4. Choose a stack name (15 characters or fewer to avoid ALB/target group name
   limits) and create the stack.
5. If ACM validation is stuck, verify the CNAME record in Route53.
6. Point your domain to the ALB DNS name (create an ALIAS or CNAME).
7. Wait for stack completion, then open `https://<your-domain>:444`.

Cockpit is available at `https://<your-domain>/cockpit`.

## CloudFormation Template Hosting (S3 Required)

AWS CloudFormation requires `templateURL` to point to an S3-hosted file. This
repo publishes `infra/aws/cloudformation` to S3 via GitHub Actions so the Launch
Stack button always references a public S3 URL.

### Fork Setup (One-Time)

1. **Create an S3 bucket** for templates (example: `opencode-cloud-templates`).
2. **Allow public reads** for the template prefix (or use signed URLs). Minimal
   bucket policy example:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PublicReadCloudFormationTemplates",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::YOUR_BUCKET/cloudformation/*"
    }
  ]
}
```

3. **Create GitHub OIDC access** in AWS (recommended):
   - Create an IAM OIDC provider for `https://token.actions.githubusercontent.com`.
   - Create a role that trusts your repo (`repo:ORG/REPO:*`), grants S3 write
     access to the bucket/prefix, and includes `AWSCloudFormationReadOnlyAccess`
     so the workflow can run template validation from the GitHub Action
     [`publish-cloudformation.yml`](../../.github/workflows/publish-cloudformation.yml)
     and upload the templates to the bucket.
4. **Set GitHub repository secrets/vars**:
   - `AWS_ROLE_ARN` (secret)
   - `AWS_CFN_BUCKET` (variable, must match README Launch Stack URL)
   - `AWS_CFN_PREFIX` (variable, optional, default `cloudformation`)
   - `AWS_REGION` (variable, optional, default `us-east-1`)
5. **Run the workflow**: `.github/workflows/publish-cloudformation.yml` (push to
   `main` or run manually).
6. **Update the Launch Stack URL** in `README.md` and `packages/core/README.md`
   to point at your bucket:

```
https://s3.amazonaws.com/YOUR_BUCKET/cloudformation/opencode-cloud-quick.yaml
```

For non-`us-east-1` buckets, use the regional endpoint:

```
https://YOUR_BUCKET.s3.<region>.amazonaws.com/cloudformation/opencode-cloud-quick.yaml
```

## Required Parameters

- **DomainName**: Fully-qualified domain name for HTTPS. ACM requires DNS
  validation before the listener becomes active.

## Outputs

- **OpencodeUrl**: Primary HTTPS URL.
- **CockpitUrl**: Cockpit HTTPS URL (`/cockpit`).
- **AlbDnsName**: ALB DNS name for debugging and DNS setup.
- **CertificateArn**: ACM certificate ARN.
- **InstanceId**: EC2 instance ID.
- **VpcId**, **PublicSubnets**, **PrivateSubnet**: Networking details.
- **CredentialsSecretArn**: Secrets Manager ARN containing generated credentials.

## Retrieving Credentials

Credentials are generated during provisioning and stored in AWS Secrets
Manager. The stack outputs `CredentialsSecretArn`.

Fetch the secret:

```bash
aws secretsmanager get-secret-value \
  --secret-id <credentials-secret-arn> \
  --query SecretString \
  --output text
```

The secret includes the username, password, and URLs. `/etc/motd` shows the
username and secret ARN, but not the password.

## opencode-cloud CLI on the host

The quick deploy installs the `opencode-cloud` CLI via `cargo install` during
provisioning. This pulls the latest published version at deploy time (Rust
1.85+ required) and can add several minutes to first boot.

You can check the installed version on the instance:

```bash
opencode-cloud --version
```

## Advanced Parameters

### Instance

- **InstanceType**: Override the default `t3.medium`. Smaller sizes (t3/t3a
  micro or small) may be unstable under load.
- **RootVolumeSize**: Root EBS volume size in GiB (default: 30).
- **KeyPairName**: Optional. Provide an EC2 key pair to enable SSH if you still
  want shell access (credentials are available in Secrets Manager).
- **OpencodeUsername**: Customize the initial username.

### Networking

- **UseExistingVpc**: Set to `true` to deploy into an existing VPC.
- If `UseExistingVpc=false`, the stack creates a new VPC with public/private
  subnets and a NAT gateway for outbound access.
- **ExistingVpcId**: Required if using an existing VPC.
- **ExistingPublicSubnetIds**: Public subnets for the ALB (must allow internet).
- **ExistingPrivateSubnetId**: Private subnet for the instance (must have NAT
  egress so the instance can pull the container image). The stack always creates
  a NAT gateway for outbound access.
- **AllowSsh**: Enable SSH access (defaults to SSM-only).
  When enabled, SSH allows inbound traffic from 0.0.0.0/0, but the instance
  stays in a private subnet with no public IP. Access still requires a path
  into the VPC (for example, via a bastion host, VPN, or SSM).
  Note: The instance still needs outbound internet access (NAT gateway + route)
  to install packages during bootstrap.

### Gotchas

- **Existing VPC + NAT placement**: When `UseExistingVpc=true`, the stack always
  creates a NAT gateway in the first subnet in `ExistingPublicSubnetIds`. That
  subnet must be public (route to an Internet Gateway) or the NAT gateway will
  not have outbound access and package installs will fail.
- **Private subnet egress**: The private subnet must route `0.0.0.0/0` to the NAT
  gateway created by the stack so the instance can install packages and pull the
  container image.
- **No public IP by design**: The instance is always in a private subnet with
  `AssociatePublicIpAddress=false`. SSH (when enabled) still requires a VPC path
  such as SSM Session Manager, a bastion host, or VPN.
- **Stack name length**: The stack name is embedded in ALB and target group
  names to prevent naming collisions across stacks. Keep the stack name <= 15
  characters to avoid AWS name length limits.

### TLS

- **HostedZoneId**: Required. Used to create ACM DNS validation records in
  Route53.

### Image

- **OpencodeImage**: Override the container image (defaults to GHCR latest).

## Troubleshooting

- **ACM validation stuck**: Ensure the CNAME record is created exactly as shown
  in ACM and that DNS has propagated.
- **HTTPS not working**: Confirm the domain points to the ALB and the ACM
  certificate is issued.
- **UI not loading**: The UI is served over HTTPS on port 444. Make sure
  `https://<your-domain>:444` is reachable and your network allows outbound
  traffic on port 444.
- **Stack rollback during create**: The stack uses a CloudFormation
  `CreationPolicy` and `cfn-signal` from the instance bootstrap. It only
  completes if the opencode service is reachable on port 3000. If the signal is
  never sent within 30 minutes, CloudFormation rolls back and the instance
  terminates.
- **Health check logic**: After the container starts, bootstrap waits for
  `http://localhost:3000/` to respond for ~60 seconds (30 attempts with 2-second
  sleeps). Failure to reach the port signals stack failure. Review
  `/var/log/cloud-init-output.log` for details.
- **Health checks failing**: Check `/var/log/cloud-init-output.log` and run
  `docker ps` on the instance to confirm the container is running.
- **Cockpit not loading**: Verify the `/cockpit` path and that port 9090 is
  reachable from the ALB security group.

## Teardown / Uninstall

To remove all AWS resources created by the quick deploy:

1. **Delete the CloudFormation stack** from the AWS console or CLI. This
   removes the ALB, EC2 instance, security groups, and networking created by
   the stack.
2. **Remove DNS records** you created for the domain:
   - Delete the ALIAS/CNAME that points to the ALB.
   - Delete the ACM validation CNAME if you added it manually.
3. **Check ACM certificates** (optional): if you requested a certificate
   outside the stack, remove it manually.

If you deployed into an existing VPC or subnets, those shared resources are
not deleted when the stack is removed.
