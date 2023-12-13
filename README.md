<h1>Build From Source (Option 1)</h1>

```shell (~project)
cargo build
cargo run
cargo build --release
./target/release/bellman_ford_pegasus
  # or for windows...
./target/release/bellman_ford_pegasus.exe
```

<h1>Build From Binary with Docker (Option 2 - AWS)</h1>

### Setup EC2

Instance Settings:

Name: cw-zscore-scanner
OS: Ubuntu 22.04
Architecture: x86
Spec: t3.small
Volume: 8GB

Security Groups:
allow-ssh (port 22 SSH)

IAM:
cw-ec2-S3-full-control

```shell (user data)
#!/bin/bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential curl wget git ufw nginx pkg-config libssl-dev awscli redis-tools
```

### Build for AWS EC2

Open up docker on your machine and create an x86 binary file for Ubuntu:

```shell (~project)
docker buildx build --platform linux/amd64 -t bellman_ford_pegasus .
docker create --name temp-container bellman_ford_pegasus
docker cp temp-container:/usr/src/myapp/target/release/bellman_ford_pegasus .
docker rm temp-container
```

### Upload to AWS S3

Upload the binary saved in the bellman_ford_pegasus folder to the AWS S3 bucket

### Add .ENV To Root

```shell
cd / && sudo touch .env && sudo nano .env
```

### Copy Binary to EC2

Extract and test webserver.

```shell
cd ~
aws s3 cp s3://<YOUR-BUCKET>/bellman_ford_pegasus /home/ubuntu/bellman_ford_pegasus
chmod +x bellman_ford_pegasus
./bellman_ford_pegasus
```

<h1>Setup CRON</h1>

### Run CRON

Run job periodically every hour

```shell
crontab -e
```

```cron
0 * * * * /bin/timeout -s 2 3595 /home/ubuntu/bellman_ford_pegasus >output_day.txt  2>&1
```
