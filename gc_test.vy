fn leak_memory() {
    let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let s = string_to_bytes("Hello, World!");
    return 0;
}

fn main() {
    print("Starting GC Test");
    let i = 0;
    while i < 10000 {
        leak_memory();
        i = i + 1;
    }
    print("Finished allocating 20,000 objects. Leaked memory.");
    let reclaimed = gc();
    print("Manually triggered GC.");
    print(reclaimed);
    return 0;
}
