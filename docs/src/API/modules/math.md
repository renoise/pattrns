# math { #math }
---
## Functions
### imod(index : [`integer`](../../API/builtins/integer.md), length : [`integer`](../../API/builtins/integer.md)) { #imod }
`->`[`integer`](../../API/builtins/integer.md)  

> Wrap a lua 1 based integer index into the given array/table length.
> 
> -> `(index - 1) % length + 1`
### random(m : [`integer`](../../API/builtins/integer.md), n : [`integer`](../../API/builtins/integer.md)) { #random }
`->`[`integer`](../../API/builtins/integer.md)  

> * `math.random()`: Returns a float in the range [0,1).
> * `math.random(n)`: Returns a integer in the range [1, n].
> * `math.random(m, n)`: Returns a integer in the range [m, n].
> 
> Overridden to use a `Xoshiro256PlusPlus` random number generator to ensure that
>  seeded random operations behave the same on all platforms and architectures.
> 
> [View documents](command:extension.lua.doc?["en-us/51/manual.html/pdf-math.random"])
### randomseed(x : [`integer`](../../API/builtins/integer.md)) { #randomseed }
> Sets `x` as the "seed" for the pseudo-random generator.
> 
> Overridden to seed the internally used  `Xoshiro256PlusPlus` random number generator.
> 
> [View documents](command:extension.lua.doc?["en-us/51/manual.html/pdf-math.randomseed"])
### randomstate(seed : [`integer`](../../API/builtins/integer.md)[`?`](../../API/builtins/nil.md)) { #randomstate }
`->` (m : [`integer`](../../API/builtins/integer.md)[`?`](../../API/builtins/nil.md), n : [`integer`](../../API/builtins/integer.md)[`?`](../../API/builtins/nil.md)) `->` [`number`](../../API/builtins/number.md)  

> Create a new local random number state with the given optional seed value.
> 
> When no seed value is specified, the global `math.randomseed` value is used.
> When no global seed value is available, a new unique random seed is created.
> 
> Random states can be useful to create multiple, separate seeded random number
> generators, e.g. in pattern, gate or emit generators, which get reset with the
> generator functions.
> 
> #### examples:
> 
> ```lua
> return pattern {
>   event = function(init_context)
>     -- use a unique random sequence every time the pattern gets (re)triggered
>     local rand = math.randomstate(12345)
>     return function(context)
>       if rand(1, 10) > 5 then
>         return "c5"
>       else
>         return "g4"
>       end
>   end
> }
> ```