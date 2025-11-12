# basecalc: Your Towel in the Mathematical Cosmos

Welcome to basecalc, the ultimate command-line calculator for intergalactic mathematicians, quantum physicists, and anyone who's ever needed to split the bill at the Restaurant at the End of the Universe!

```
 _                              _      
| |                            | |     
| |__   __ _ ___  ___  ___ __ _| | ___ 
| '_ \ / _` / __|/ _ \/ __/ _` | |/ __|
| |_) | (_| \__ \  __/ (_| (_| | | (__ 
|_.__/ \__,_|___/\___|\___\__,_|_|\___|   
```

## 🚀 Key Features

- **Arbitrary Base Calculations**: From Binary to Hexatrigesimal (base 2 to Z+1), because who knows what number system the others use?
- **Complex Number Wizardry**: Juggle real and imaginary numbers like a cosmic jester.
- **Precision to Rival a Neutrino Detector**: Adjustable digit precision for when you absolutely need to know the 1000th digit of pi in base 7.
- **Trigonometric Functions**: Calculate the waves needed for your intergalactic surfing adventures.
- **Constants at Your Fingertips**: π, e, and other mathematical celebrities are always at the ready.
- **Previous Result Recall**: Use '&' to reference your last calculation, perfect for building fractals in the terminal.
- **VSF Integration**: State-of-the-art data storage and retrieval using the Versatile Storage Format.

## 🆕 VSF Integration

basecalc now incorporates the Versatile Storage Format (VSF) for state and history storage. This marks the first public appearance of VSF!

### What is VSF?

VSF (Versatile Storage Format) is designed for efficiency, security, and adaptability. It provides a complete and unified solution for storing and managing any type of data, from simple values to complex structures.

Key features of VSF include:
- Optimized for efficiency and compact size
- Built-in security and validity checks
- Transparent data exchange
- Unified metadata framework
- Spectral accuracy in colour and data representation
- Proof of authenticity and chain of trust
- Future-proof design for technological advances

In basecalc, VSF is used to store the calculator's state and history. Take a look and put VSF to use in your projects!

For more information about VSF, visit [https://sunsarrow.com/vsf](https://sunsarrow.com/vsf) and [https://github.com/nickspiker/vsf](https://github.com/nickspiker/vsf)

## 🧮 How to Use

1. Launch basecalc
2. Type your mathematical musings
3. Press Enter
4. Marvel at the results
5. Repeat until you've solved all of the universe's mysteries (or just your homework)

## 🔢 Entering Numbers

Numbers in basecalc are like tribbles - they come in all shapes and sizes. At least all that are allowed for 0-9 plus A-Z:

- Regular numbers: `42`, `@pi`, `4R3.6A74cg7FR`
- Complex numbers: `[3, 4]` (That's 3 + 4i for you Earth-dwellers)

Spaces, tabs, and underscores are ignored, so feel free to make your numbers as readable as a Vogon poetry book.

## 🎛️ Commands

- `:base <digit>`: Switch bases faster than a Time Lord switches regenerations. Works for & too.
- `:digits <value>`: Adjust precision because sometimes you need more than 42 digits.
- `:radians` / `:degrees`: Toggle between radians and degrees, useful for both interstellar navigation and pizza slicing.
- `:help`: Summon the Guide (that's me!) for assistance.
- `:debug`: Peek behind the curtain of the mathematical matrix.
- `:test`: Ensure your calculator isn't suffering from a Babelfish infestation.

## 🧠 Operators and Functions

### Basic Operators
- `+`, `-`, `*`, `/`: The fantastic four of arithmetic.
- `^`: Exponentiation, for when your numbers need to reach for the stars.
- `%`: Modulus, because even the universe has leftovers.

### Unary Operators
- `#abs`: Absolute value, for numbers with identity crises.
- `#sqrt`: Square root, the mathematical equivalent of splitting an atom.
- `#ln`, `#log`: Natural and current base logarithms, for when your numbers need to get down to earth.
- `#sin`, `#cos`, `#tan`: Trigonometric functions, essential for surfing thru spacetime.
- `#asin`, `#acos`, `#atan`: Inverse trig, for when you need to undo your sinful calculations.
- `#ceil`, `#floor`, `#round`: For when you need to flatten the curve of your results.
- `#re`, `#im`: Extract real and imaginary parts, like separating Siamese twins.

### Constants
- `@pi`: π, the circle's best friend.
- `@e`: e, the natural choice for exponential explorers.
- `@gamma`: The Euler-Mascheroni constant, for those who like their math extra crispy.
- `@rand`: Random number generator, for when you need to simulate uncertainty.
- `@grand`: Gaussian random number, because sometimes your randomness needs a bell curve.

## 🌟 Examples

```
> 2 + 3 * 4
  14

> #sin(@pi/4)
  0.707106781187

> [3, 4] * [1, -1]
  [7, 1]

> #sqrt-1
  [0, 1]

> :base C
Base set to Dozenal (C).

> 5^-25*[-3.24,-4.1b]
  [-5.58 BA6 424 28A 6A9 238 829 27A~ :-17, -7.17 49A 618 591 429 757 6B6 512~ :-17]

> :base A
Base set to Decimal (A).

> 9+1
  10
```

Thank you for providing the content to rework. I'll revise the "Building and Running" section to make it more accessible for those who aren't familiar with Rust. Here's the updated version:

#### 🚀 Building and Running

1. **Install Rust**: 
   If you haven't already, you'll need to install Rust on your system. Visit the official Rust website (https://www.rust-lang.org/) and follow the installation instructions for your operating system.

2. **Clone the repository**:
   Open your terminal or command prompt and run:
   ```
   git clone https://github.com/nickspiker/basecalc.git
   ```
   This downloads the basecalc source code to your machin.

3. **Navigate to the basecalc directory**:
   Change to the newly created basecalc directory:
   ```
   cd basecalc
   ```

4. **Build and run basecalc**:
   Now, use Rust's package manager, Cargo, to build and run the application:
   ```
   cargo run --release
   ```
   This command compiles the code and starts basecalc. The `--release` flag ensures optimal performance.

5. **Start calculating**:
   Once basecalc is running, you can start entering calculations and commands!

**For non-Rust users**:
If you're not comfortable with building from source, keep an eye on the project's GitHub page. In the future, pre-compiled binaries will be available for different operating systems, making it easier to run basecalc without needing to install Rust or compile yourself.

## 🌌 Contributing

Found a bug? Have an idea for a feature that would make basecalc even more cosmic? Contributions are welcome! Open an issue or submit a pull request, and let's make basecalc the calculator that even Deep Thought would be jealous of.

## 📜 License

Basecalc is free software, just like the answer to life, the universe, and everything. It's licensed under the GNU General Public License v3.0 (GPLv3), because we believe in the power of open source to drive innovation across the galaxy.

---

Remember, DON'T PANIC, and always know where your basecalc is! Happy calculating, and may your computations be swift and your errors be few!
