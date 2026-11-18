# Third-party notices

CG itself is under the MIT licence (`LICENSE`). The parts below come from others.

## Pokémon Showdown (game data)

CG's game data (`data/`, `engine/src/sim/data/generated.rs`) is generated from Pokémon Showdown's
`champions` mod at commit `c0845f33d381402dfcb18ce324c20a94f18af075`
(https://github.com/smogon/pokemon-showdown), and CG's rules were written from its source.

```
The MIT License (MIT)

Copyright (c) 2011-2026 Guangcong Luo and other contributors http://pokemonshowdown.com/

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

## Rust crates compiled into the extension (`_cg`)

| Crate | Version | Licence |
|---|---|---|
| pyo3 | 0.29.3 | MIT OR Apache-2.0 |
| pyo3-ffi | 0.29.3 | MIT OR Apache-2.0 |
| libc | 0.2.155 | MIT OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |

The Rust standard library (MIT OR Apache-2.0) and llvm-mingw's compiler-rt and libunwind (Apache-2.0 WITH
LLVM-exception) are linked in as well.
