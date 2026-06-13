use crate::vm::memory::Heap;
use crate::vm::value::Value;

pub fn get(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("http.get expects exactly 1 argument (url)".to_string());
    }

    let url = if let Value::String(u) = &args[0] {
        u
    } else {
        return Err("http.get url must be a string".to_string());
    };

    match ureq::get(url).call() {
        Ok(response) => {
            match response.into_string() {
                Ok(body) => Ok(Value::String(body)),
                Err(e) => Err(format!("http.get failed to read response body: {}", e)),
            }
        }
        Err(e) => Err(format!("http.get failed: {}", e)),
    }
}

pub fn post(_heap: &mut Heap, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("http.post expects exactly 2 arguments (url, body)".to_string());
    }

    let url = if let Value::String(u) = &args[0] {
        u
    } else {
        return Err("http.post url must be a string".to_string());
    };

    let body = if let Value::String(b) = &args[1] {
        b
    } else {
        return Err("http.post body must be a string".to_string());
    };

    match ureq::post(url).send_string(body) {
        Ok(response) => {
            match response.into_string() {
                Ok(resp_body) => Ok(Value::String(resp_body)),
                Err(e) => Err(format!("http.post failed to read response body: {}", e)),
            }
        }
        Err(e) => Err(format!("http.post failed: {}", e)),
    }
}
