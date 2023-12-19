<h1>Build From Source</h1>

After downloading project, cd into project and...

```shell (~project)
cargo build
cargo run
cargo build --release
./target/release/bellman_ford_pegasus
  # or for windows...
./target/release/bellman_ford_pegasus.exe
```

### Create .env

Create a .env file in the project folder

```shell (~project)
touch .env
```

Add your Binance API Key and Secret. Ensure that these are marked for SPOT trading by restricting to your IP address and checking the relevant enable of spot trading checkbox.

```conf (~env)
BINANCE_API_KEY=ENTER YOUR KEY HERE
BINANCE_API_SECRET=ENTER YOUR SECRET HERE
```
