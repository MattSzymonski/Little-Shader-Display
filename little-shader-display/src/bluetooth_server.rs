use bluer::rfcomm::{Listener, SocketAddr};
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use std::sync::Arc;

pub struct BluetoothServer {
    pub received_text: Arc<Mutex<Option<String>>>,
}

impl BluetoothServer {
    pub async fn new() -> bluer::Result<Self> {
        Ok(BluetoothServer {
            received_text: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn run(&self,) -> bluer::Result<()> {
        println!("Starting Bluetooth server...");

        // Create a new Bluetooth session
        let session = bluer::Session::new().await?;

        // Get the default Bluetooth adapter
        let adapter = session.default_adapter().await?;

        // Enable the Bluetooth adapter
        adapter.set_powered(true).await?;

        // Make the device discoverable to others
        adapter.set_discoverable(true).await?;

        // Get the adapter's Bluetooth address
        let adapter_addr = adapter.address().await?;

        // Create a Bluetooth RFCOMM socket address (channel 1)
        let local_sa = SocketAddr::new(adapter_addr, 1);

        // Bind an RFCOMM listener to the address
        let listener = Listener::bind(local_sa).await?;

        // Log server address and channel info
        println!(
            "Listening on {} channel {}. Press enter to quit.",
            listener.as_ref().local_addr()?.addr,
            listener.as_ref().local_addr()?.channel
        );

        // Create a buffered reader to read from stdin for quit detection
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();

        loop {
            println!("\nWaiting for connection...");

            // Wait for a client to connect or user input to quit
            let (mut stream, sa) = tokio::select! {
                l = listener.accept() => {
                    match l {
                        Ok(v) => v,
                        Err(err) => {
                            println!("Accepting connection failed: {}", &err);
                            continue;
                        }
                    }
                },
                _ = lines.next_line() => break,
            };

            println!("Accepted connection from {:?}", &sa);

            // Send a greeting message to the connected client
            println!("Sending hello");
            if let Err(err) = stream.write_all("Hello from rfcomm_server!".as_bytes()).await {
                println!("Write failed: {}", &err);
                continue;
            }

            let mut read_buffer = vec![0; 1024];
            let mut message_buffer = String::new();

            loop {
                match stream.read(&mut read_buffer).await {
                    Ok(0) => {
                        println!("Client disconnected.");
                        break;
                    }
                    Ok(n) => {
                        // Decode and append to the message buffer
                        if let Ok(text) = std::str::from_utf8(&read_buffer[..n]) {
                            message_buffer.push_str(text);

                            // Process each complete message line
                            while let Some(idx) = message_buffer.find('\n') {
                                // Get line without \n
                                let line = message_buffer[..idx].trim();

                                // Store the complete line text in the mutex so other code can access it
                                *self.received_text.lock().await = Some(line.to_string());

                                // Remove processed line from the buffer
                                message_buffer = message_buffer[idx + 1..].to_string();
                            }
                        } else {
                            message_buffer.clear();
                        }
                    }
                    Err(err) => {
                        println!("Read failed: {}", err);
                        break;
                    }
                }
            }

            // Close the stream after processing
            if let Err(err) = stream.shutdown().await {
                println!("Shutdown failed: {}", &err);
            }
            println!("Connection closed.");
        }

        Ok(())
    }
}