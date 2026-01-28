# AWS Quick Deploy (One-Click)

Deploy opencode-cloud on AWS with a public Application Load Balancer (ALB) and
HTTPS via ACM, while keeping the EC2 instance private by default.

## Prerequisites

- AWS account with permissions to create EC2, ALB, ACM, and IAM resources.
- A domain name you control (required for ACM TLS validation).
- Ability to edit DNS records for the domain.

## Quick Deploy

1. Click the AWS deploy button in the root `README.md`.
2. Provide a **domain name** (e.g., `opencode.example.com`).
3. (Optional) Provide a Route53 hosted zone ID for automatic DNS validation.
4. Create the stack.
5. If ACM validation is not automatic, add the CNAME record shown in ACM.
6. Point your domain to the ALB DNS name (create an ALIAS or CNAME).
7. Wait for stack completion, then open `https://<your-domain>`.

Cockpit is available at `https://<your-domain>/cockpit`.

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
- **CredentialsFile**: Location of generated credentials on the instance.

## Retrieving Credentials

Credentials are generated during provisioning and saved on the instance at:

```
/var/lib/opencode-cloud/deploy-status.json
```

Use AWS Systems Manager Session Manager to access the instance:

```bash
aws ssm start-session --target <instance-id>
sudo cat /var/lib/opencode-cloud/deploy-status.json
```

The file includes the username, password, and URLs. The same details are also
present in `/etc/motd`.

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
- **KeyPairName**: Provide an EC2 key pair to enable SSH.
- **OpencodeUsername**: Customize the initial username.

### Networking

- **UseExistingVpc**: Set to `true` to deploy into an existing VPC.
- If `UseExistingVpc=false`, the stack creates a new VPC with public/private
  subnets and a NAT gateway for outbound access.
- **ExistingVpcId**: Required if using an existing VPC.
- **ExistingPublicSubnetIds**: Public subnets for the ALB (must allow internet).
- **ExistingPrivateSubnetId**: Private subnet for the instance (must have NAT
  egress so the instance can pull the container image).
- **AllowSsh**: Enable SSH access (defaults to SSM-only).
- **SshIngressCidr**: Limit SSH access to a trusted CIDR range.

### TLS

- **HostedZoneId**: If provided, ACM DNS validation is automated via Route53.

### Image

- **OpencodeImage**: Override the container image (defaults to GHCR latest).

## Troubleshooting

- **ACM validation stuck**: Ensure the CNAME record is created exactly as shown
  in ACM and that DNS has propagated.
- **HTTPS not working**: Confirm the domain points to the ALB and the ACM
  certificate is issued.
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
