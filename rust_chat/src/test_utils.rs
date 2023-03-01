use rand::{Rng, SeedableRng};
use std::io::Write;

static PORT_COUNTER: std::sync::Mutex<std::cell::RefCell<i32>> =
    std::sync::Mutex::new(std::cell::RefCell::new(5432));

pub fn get_next_port() -> i32 {
    let guard = PORT_COUNTER.lock().expect("");
    let mut value_ref = guard.borrow_mut();
    let previous_value = *value_ref;
    *value_ref += 1;
    previous_value
}

pub fn next_localhost_address() -> String {
    format!("127.0.0.1:{}", get_next_port())
}

pub fn make_buffer_for_packet(payload: &str) -> Vec<u8> {
    assert_ne!(payload.len(), 0);
    let len: u32 = payload.len().try_into().expect("");
    let slice = unsafe {
        let pointer = &len as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, 4)
    };
    let mut buffer: Vec<u8> = Vec::new();
    buffer
        .write(&slice)
        .expect("Failed to write size to buffer");
    buffer
        .write(payload.as_bytes())
        .expect("Failed to write payload to buffer");
    buffer
}

pub fn generate_random_string(seed: u64, min_length: usize, max_length: usize) -> String {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let length = rng.gen_range(min_length..max_length + 1);

    (0..length)
        .map(|_| rng.gen_range(b'a'..b'z' + 1) as char)
        .collect()
}
