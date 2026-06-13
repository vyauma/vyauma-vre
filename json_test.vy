import "stdlib/json.vy"

fn main() {
    let raw = "{\"user\": \"vyauma_dev\", \"active\": true, \"scores\": [10, 20, 30], \"metadata\": {\"id\": 999}}";
    print("Parsing JSON:");
    print(raw);
    
    let parsed = json_parse(raw);
    
    // Demonstrate interoperability with dictionaries
    print("User:");
    print(parsed["user"]);
    print("Active Status:");
    print(parsed["active"]);
    print("First Score:");
    print(parsed["scores"][0]);
    print("Metadata ID:");
    print(parsed["metadata"]["id"]);

    // Modify the parsed JSON
    let scores = parsed["scores"];
    scores[1] = 99;
    parsed["user"] = "vyauma_master";

    let re_stringified = json_stringify(parsed);
    print("Re-stringified JSON:");
    print(re_stringified);
}
