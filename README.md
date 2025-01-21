# TON Shard Master CLI Tool


TON Shard Master CLI Tool is a command-line application for working with the TON blockchain. 

It allows you to:

- Generate TON accounts with a specific shard. 
- Detect the shard for a given address.

## Installation

### 1. Clone the repository

```bash
git clone
```

### 2. Install dependencies

```bash
cd shard-master
cargo build --release
```

### 3. Run the application

```bash
./target/release/shard-master
```

or 

```bash
 cargo install --git <path to the repository>
```



## Usage
### 1. Generate Command

Use the `generate` command to create a new mnemonic and account, and specify the shard using the --shard option.

```bash
./shard-master generate --shard <shard>
```
### 2. Shard Command

Use the shard command to detect the shard of a given address.

```bash
./shard-master shard <address>
```

## Help

To see the available commands and options:
    
```bash
./shard-master --help
```

## Disclaimer

### This application is provided for `educational purposes only`. We are not responsible for any misuse, loss, or damage caused by using this software. Use at your own risk.

## License

This project is licensed under the MIT License. See the LICENSE file for details.
