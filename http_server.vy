fn main() {
    print("Starting Vyauma HTTP Server on port 8080...");
    let server = net_listen(8080);
    
    if server < 0 {
        print("Failed to bind to port 8080.");
        return 0;
    }
    
    // HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!
    let res = [
        72, 84, 84, 80, 47, 49, 46, 49, 32, 50, 48, 48, 32, 79, 75, 13, 10, 
        67, 111, 110, 116, 101, 110, 116, 45, 76, 101, 110, 103, 116, 104, 58, 32, 49, 51, 13, 10, 
        13, 10, 
        72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33
    ];

    print("Server is listening. Point your browser to http://localhost:8080");

    while 1 == 1 {
        print("Looping, going to accept...");
        let client = net_accept(server);
        print("Accept returned!");
        
        if client > 0 {
            // Read some data just to drain the socket briefly
            let buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let r = read(client, buf);
            
            print("Accepted a new connection! Sending HTTP response...");
            write(client, res);
            close(client);
        }
    }
}
