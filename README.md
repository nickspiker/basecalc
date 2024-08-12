# Basecalc: Your Towel in the Mathematical Cosmos

Copyright (C) 2023 Nick Spiker

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

# Basecalc: Your Towel in the Mathematical Cosmos

Welcome to Basecalc, the ultimate command-line calculator for intergalactic mathematicians, quantum physicists, and anyone who's ever wondered how to split the bill at the Restaurant at the End of the Universe!

```
 ____                           _      
|  _ \                         | |     
| |_) | __ _ ___  ___  ___ __ _| | ___ 
|  _ < / _` / __|/ _ \/ __/ _` | |/ __|
| |_) | (_| \__ \  __/ (_| (_| | | (__ 
|____/ \__,_|___/\___|\___\__,_|_|\___|
```

## 🚀 Key Features

- **Arbitrary Base Calculations**: From Binary to Hexatrigesimal (base 2 to Z+1), because who knows what number system non humans use?
- **Complex Number Wizardry**: Juggle real and imaginary numbers like a cosmic jester.
- **Precision to Rival a Neutrino Detector**: Adjustable digit precision for when you absolutely need to know the 1000th digit of pi in base 7.
- **Trigonometric Functions**: Calculate sine waves for your intergalactic surfing adventures.
- **Constants at Your Fingertips**: π, e, and other mathematical celebrities are always ready to party.
- **Previous Result Recall**: Use '&' to reference your last calculation, perfect for building fractals in the terminal.

## 🧮 How to Use

1. Launch Basecalc
2. Type your mathematical musings
3. Press Enter
4. Marvel at the results
5. Repeat until you've solved all of the universe's mysteries (or just your homework)

## 🔢 Entering Numbers

Numbers in Basecalc are like tribbles - they come in all shapes and sizes. At least all that are allowed for 0-9 plus A-Z:

- Regular numbers: `42`, `@pi`, `4R3.6A74cg7FF`
- Complex numbers: `[3, 4]` (That's 3 + 4i for you Earth-dwellers)

Spaces, tabs, and underscores are ignored, so feel free to make your numbers as readable as a Vogon poetry book.

## 🎛️ Commands

- `:base <digit>`: Switch bases faster than a Time Lord switches regenerations. Works for & too.
- `:digits <value>`: Adjust precision because sometimes you need more than 42 digits.
- `:radians` / `:degrees`: Toggle between radians and degrees, useful for both interstellar navigation and pizza slicing.
- `:help`: Summon the Guide (that's me!) for assistance.
- `:debug`: Peek behind the curtain of the mathematical matrix.
- `:test`: Ensure your calculator isn't suffering from a Babel fish infestation.

## 🧠 Operators and Functions

### Basic Operators
- `+`, `-`, `*`, `/`: The fantastic four of arithmetic.
- `^`: Exponentiation, for when your numbers need to reach for the stars.
- `%`: Modulus, because even the universe has leftovers.

### Unary Operators
- `#abs`: Absolute value, for numbers with identity crises.
- `#sqrt`: Square root, the mathematical equivalent of splitting an atom.
- `#ln`, `#log`: Natural and base logarithms, for when your numbers need to get down to earth.
- `#sin`, `#cos`, `#tan`: Trigonometric functions, essential for wave-surfing through spacetime.
- `#asin`, `#acos`, `#atan`: Inverse trig, for when you need to undo your sinful calculations.
- `#ceil`, `#floor`, `#round`: For when you need to flatten the curve of your results.
- `#re`, `#im`: Extract real and imaginary parts, like separating Siamese twins.

### Constants
- `@pi`: π, the circle's best friend.
- `@e`: e, the natural choice for exponential explorers.
- `@gamma`: The Euler-Mascheroni constant, for those who like their math extra crispy.
- `@rand`: Random number generator, for when you need to simulate quantum uncertainty.
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

1. Clone this repository faster than light speed:
   ```
   git clone https://github.com/yourusername/basecalc.git
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

Found a bug? Have an idea for a feature that would make Basecalc even more cosmic? Contributions are welcome! Open an issue or submit a pull request, and let's make Basecalc the calculator that even Deep Thought would be jealous of.

## 📜 License

Basecalc is free software, just like the answer to life, the universe, and everything. It's licensed under [insert your license here], because sharing is caring, especially when it comes to interdimensional calculators.

---

Remember, DON'T PANIC, and always know where your Basecalc is! Happy calculating, and may your computations be swift and your errors be few!