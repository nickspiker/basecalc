# basecalc: Arbitrary-Base Complex Number Calculator

`basecalc` is a command-line calculator designed for handling complex numbers with large numerical precision.

## Key Features

- **Support for Arbitrary Bases**: Perform calculations in any positive integer base from binary to Z+1.
- **Complex Number Calculations**: Easily handles real and imaginary numbers.
- **High Precision**: Unrestricted digit precision for accurate and reliable results.
- **Whitespace Ignoring**: Automatically ignores tabs, spaces, and underscores.
- **Operator Support**: Includes all the basic numerical operations.
- **History Tracking**: Access previous calculations with history references. (eventually)

## Entering Numbers

Numbers can be entered in various formats, ignoring spaces, tabs, and underscores:

- Decimal and Fractional: `123.45AB`, `6a8G12.9`, `.6c`, `000.A2`
- Complex Numbers: `[123,456]`, `[real,imaginary]`

## Operators

`basecalc` supports a range of operators and mathematical functions:

### Arithmetic Operators
- `+` - Addition
- `-` - Subtraction
- `*` - Multiplication
- `/` - Division
- `%` - Modulus
- `^` - Exponentiation

### Mathematical Functions
- `#abs` - Absolute Value
- `#sqrt` - Square Root
- `#exp` - Exponential
- `#ln` - Natural Logarithm
- `#erf` - Error Function

### Trigonometric Functions in Radians
- `#sin` - Sine
- `#cos` - Cosine
- `#tan` - Tangent
- `#asin` - Inverse Sine
- `#acos` - Inverse Cosine
- `#atan` - Inverse Tangent

### Strange
- `!` - Factorial

- Prefix functions with `#` to avoid conflicts with numerical entries (e.g., `#sin` for sine).
- History references: Use `@` followed by the history line number (e.g., `@6A`).

## Constants

`basecalc` supports various mathematical constants, which can be used directly in calculations:

- `$pi` - Pi (π), the ratio of the circumference of a circle to its diameter.
- `$e` - Euler's number (e), the base of natural logarithms and other good things.
- `$phi` - The golden ratio (φ).

You can use these constants in your calculations just like any other number. For instance, to calculate the area of a circle with radius 5, you would input `5 + 5 * $pi`.