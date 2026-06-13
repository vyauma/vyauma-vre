import "stdlib/net.vy"
import "stdlib/io.vy"
import "stdlib/string.vy"

fn main() {
    let port = 8082;
    let server = create_server(port);
    if server == 0 {
        print("Failed to bind server");
        return 1;
    }
    
    set_nonblocking(server);
    
    let max_clients = 10;
    let clients = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let i = 0;
    while i < max_clients {
        clients[i] = 0;
        i = i + 1;
    }
    
    print("Async server (with stdlib) listening on port 8082...");
    
    let running = 1;
    while running == 1 {
        let new_client = accept_client(server);
        if new_client != 0 {
            print("New client connected!");
            set_nonblocking(new_client);
            
            let slot = 0;
            let found = 0;
            while slot < max_clients {
                if found == 0 {
                    if clients[slot] == 0 {
                        clients[slot] = new_client;
                        found = 1;
                    }
                }
                slot = slot + 1;
            }
            if found == 0 {
                print("Too many clients, closing connection.");
                close_conn(new_client);
            }
        }
        
        let c_idx = 0;
        while c_idx < max_clients {
            let client = clients[c_idx];
            if client != 0 {
                let data = read_all(client);
                if data != "" {
                    // We got data, echo it back
                    let response = "HTTP/1.1 200 OK\nContent-Type: text/plain\n\nHello from StdLib Async Vyauma!\n";
                    write_all(client, response);
                    close_conn(client);
                    clients[c_idx] = 0;
                    print("Handled request.");
                } else {
                    // How do we detect EOF with stdlib? 
                    // Let's assume empty data from nonblocking socket means nothing yet. 
                    // Actually, read_all returns "" on both EOF and WouldBlock.
                    // This is a flaw in our `io.vy` design, but fine for this simple demo.
                }
            }
            c_idx = c_idx + 1;
        }
        
        sleep(10);
        gc(); // Trigger GC manually to test
    }
    
    return 0;
}
