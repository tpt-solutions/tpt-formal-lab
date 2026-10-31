use out_verify_macros::ensures;

#[ensures(|a, b| a == b)]
fn f() -> i32 {
    1
}

fn main() {}
