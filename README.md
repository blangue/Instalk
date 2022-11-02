# Instalk

Instalk is a script to be installed on a TCP server. It allows to launch an instant chat server. It is entirely written in Rust.

## Build
To build the project, you will first need to install Rust and Cargo. Then, you just have to launch the following command in the project folder to build the project:

```bash
cargo build
```

The build should end successfully.

To start the server, you need to run the following command in the project folder:

```bash
cargo run <PORT_TO_LAUNCH_SERVER_ON>
```

Once launched, clients can connect to the IP and port of the machine the server runs on.
For instance on Linux machines:

```bash
nc <IP_TO_CONNECT_TO> <PORT_TO_CONNECT_TO>
```

Or launch other command prompt on the server using 127.0.0.1 as the ip address</br></br>

---

Credits can be found in credits.md file.
