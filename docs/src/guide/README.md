# Guide

A `Pattern` is the main building block in pattrns. It lets you define when and what to play.

pattrns consumes [Lua script](https://www.lua.org/) files that define patterns as specified in the [API documentation](../API/).

## Components

- [TimeBase](./timebase.md) defines the **time unit** of a pattern.
- [Pulse](./pulse.md) → [Gate](./gate.md) → [Event](./event.md) perform the basic **event generation** in 3 stages.
- [Parameters](./parameters.md) allow changing behavior of components during runtime.

All content in patterns can be either **static** or **dynamic**:

- **Static** content is defined as a Lua table. There are various helpers included in the API, such as [note scales, chords](./notes&scales.md), and [pulse table](../API/pulse.md) to ease creating static content.

- **Dynamic** content is generated on the fly by Lua functions while the pattern runs. [Generators](../extras/generators.md) are functions with local state, which can e.g. be used to apply replicable [randomization](../extras/randomization.md).

**Cycle** events use Tidal [Cycles](./cycles.md) mini-notation to create patterns in a flexible, condensed form.

## Examples

See [Quickstart](../quickstart.md) for a set of simple examples to start and to play around with. 

The [Examples](../examples/) section contains more advanced and guided examples. 
