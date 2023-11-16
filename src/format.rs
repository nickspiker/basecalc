fn main() {
    let mut operators = [
        // (Text entry, Operator, Number of operands)
        // Operators must be sorted in ASCII order!
        // ("", 0, true),          // Clear register and load number
        ("#abs", 'a', false),      // Absolute value
        ("#acos", 'C', false),     // Inverse cosine
        ("#asin", 'S', false),     // Inverse sine
        ("#atan", 'T', false),     // Inverse tangent
        ("#cos", 'c', false),      // Cosine
        ("#ln", 'l', false),       // Natural logarithm
        ("#log", 'L', false),      // Base logarithm
        ("#rand", 'r', false),     // Random
        ("#sin", 's', false),      // Sine
        ("#sqrt", 'q', false),     // Square root
        ("#tan", 't', false),      // Tangent
        ("%", '%', true),          // Moduland, modular order
        ("$", '$', true),          // Modulor, moduland order
        ("*", '*', true),          // Multiplication
        ("+", '+', true),          // Addition
        ("-", '-', true),          // Subtraction
        ("/", '/', true),          // Dividend divisor order
        ("\\", '\\', true),        // Divisor dividend order
        (":precision", 'p', true), // Sets precision in digits in given base plus 32 bits of padding
        (":base", 'b', true),      // Sets base to any base from 2 to 36
        ("?", '@', true),          // History entry
        ("@pi", 'p', true),        // Pi
        ("@e", 'e', true),         // e
        ("^", '^', true),          // Exponentiation
        ("&", '&', true),          // Exponentiation
    ];
    // Sort the operators array by the first element of each tuple
    operators.sort_by_key(|k| k.0);

    // Now print the sorted operators
    println!("{:?}", operators);
}
