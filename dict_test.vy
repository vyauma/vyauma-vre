struct Person {
    name,
    age
}

fn main() {
    print("Testing Dictionaries and Struct interop...");

    // Initialize using Dict syntax
    let map = {
        "key1": 100,
        "key2": "hello world",
        "nested": {
            "inner": 42
        }
    };

    print(map["key1"]);
    print(map["key2"]);
    
    // Dynamic access
    let k = "key2";
    print(map[k]);

    // Nested access
    print(map["nested"]["inner"]);

    // Modifying dictionary dynamically
    map["new_key"] = 999;
    print(map["new_key"]);

    // Interop with Struct syntax
    let p = new Person {
        name: "Alice",
        age: 30
    };

    print("Struct field accessed via dot:");
    print(p.name);
    
    print("Struct field accessed via dict index:");
    print(p["age"]);

    // Assign via dict index, read via dot
    p["job"] = "Engineer";
    print(p.job);

    // Dict dot notation (treating dict like a struct)
    // Wait, parser restricts property names to identifiers. So `map.key1` should work!
    print("Dict accessed via dot notation:");
    print(map.key1);
    
    return 0;
}
