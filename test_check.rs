use naso_compiler::{parse_program, check_program};

fn main() {
    let source = r#"
        fn test() {
            let [1] x = 42;
            let y = x;
        }
    "#;
    let mut program = parse_program(source).expect("Failed to parse");
    let result = check_program(&mut program);
    if result.errors.is_empty() {
        println!("OK");
    } else {
        for e in result.errors {
            println!("ERROR: {:?}", e);
        }
    }
}
