fn main() {
    let s = "Hello, Vyauma!";
    let bytes = string_to_bytes(s);
    let s2 = bytes_to_string(bytes);
    print(s2);
    return 0;
}
