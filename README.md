# basecalc: Your Towel in the Mathematical Cosmos

Copyright (C) 2024 Nick Spiker

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

---

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

## 🚀 Installation

1. Clone this repository as fast as light:
   ```
   git clone https://github.com/nickspiker/basecalc.git
   ```
2. Navigate to the basecalc directory:
   ```
   cd basecalc
   ```
3. Build the project with cargo:
   ```
   cargo build --release
   ```
4. Run basecalc and start your journey through the mathematical multiverse:
   ```
   cargo run --release
   ```

## 🌌 Contributing

Found a bug? Have an idea for a feature that would make basecalc even more cosmic? Contributions are welcome! Open an issue or submit a pull request, and let's make basecalc the calculator that even Deep Thought would be jealous of.

## 📜 License

Basecalc is free software, just like the answer to life, the universe, and everything. It's licensed under the GNU General Public License v3.0 (GPLv3), because we believe in the power of open source to drive innovation across the galaxy.

### What this means for you:

1. **Freedom to Use**: You can use Basecalc for any purpose, be it calculating the odds of successfully navigating an asteroid field or determining the exact amount of Pangalactic Gargleblasters needed for a party.

2. **Freedom to Study**: You have the right to study how Basecalc works and adapt it to your needs. The source code is your playground!

3. **Freedom to Share**: You can copy and distribute Basecalc to your fellow hitchhikers, ensuring that no one in the universe is without a towel or a powerful calculator.

4. **Freedom to Improve**: You can enhance Basecalc and release your improvements to the public. Maybe you'll add a function to calculate the probability of improbability?

### The Copyleft Principle:

The GPL is a copyleft license, which means that any derivative work you distribute must also be licensed under the GPL. This ensures that the freedoms are passed on to all users of Basecalc and its derivatives.

### Why GPL?

We chose the GPL because we believe that software, like knowledge, should be free and open. By using the GPL, we ensure that Basecalc remains free software, even as it evolves and improves over time.

For the full text of the license, please see the [LICENSE](LICENSE) file in the Basecalc repository. Remember, understanding software licenses is almost as important as knowing where your towel is!

---

Remember, DON'T PANIC, and always know where your basecalc is! Happy calculating, and may your computations be swift and your errors be few!
