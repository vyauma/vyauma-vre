fn main() {
    let port = 8081;
    let server = net_listen(port);
    if server == 0 - 1 {
        print("Failed to bind server");
        return 1;
    }
    
    // Put listener into non-blocking mode
    net_set_nonblocking(server, 1);
    
    let max_clients = 10;
    let clients = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let i = 0;
    while i < max_clients {
        clients[i] = 0 - 1;
        i = i + 1;
    }
    
    print("Async server listening on port 8081...");
    
    let running = 1;
    while running == 1 {
        let new_client = net_accept(server);
        if new_client != 0 - 1 {
            print("New client connected!");
            net_set_nonblocking(new_client, 1);
            
            // Find a free slot
            let slot = 0;
            let found = 0;
            while slot < max_clients {
                if found == 0 {
                    if clients[slot] == 0 - 1 {
                        clients[slot] = new_client;
                        found = 1;
                    }
                }
                slot = slot + 1;
            }
            if found == 0 {
                print("Too many clients, closing connection.");
                close(new_client);
            }
        }
        
        let c_idx = 0;
        while c_idx < max_clients {
            let client_fd = clients[c_idx];
            if client_fd != 0 - 1 {
                let buf = string_to_bytes("                                                                                                                                ");
                let bytes_read = read(client_fd, buf);
                
                if bytes_read > 0 {
                    // We got data, echo it back via HTTP response
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from Async Vyauma Server!\n";
                    let res_bytes = string_to_bytes(response);
                    write(client_fd, res_bytes);
                    close(client_fd);
                    clients[c_idx] = 0 - 1;
                    print("Handled request.");
                } else {
                    if bytes_read == 0 {
                        // EOF
                        close(client_fd);
                        clients[c_idx] = 0 - 1;
                        print("Client disconnected.");
                    }
                }
            }
            c_idx = c_idx + 1;
        }
        
        // Sleep for 10ms to prevent busy-waiting 100% CPU lock
        sleep(10);
    }
    
    return 0;
}
