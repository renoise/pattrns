# Benchmarks

[Criterion](https://github.com/bheisler/criterion.rs) benchmarks for `pattrns`.

### Running

Benchmarks are used by a GitHub PR [workflow](../.github/workflows/benchmark.yml) to ensure performance does not regress with changes.

To run benchmarks manually:

```sh
# cd into the repository *root*
cargo bench
```

To compare against some other branches / baselines:

- Check out the master branch (or some other branch you like to compare to).
- Run the benchmarks once to establish a baseline.
- Switch to the target branch.
- Run the benchmarks a second time to compare against the baseline.

## License

`pattrns` is distributed under the terms of the [GNU Affero General Public License V3](https://www.gnu.org/licenses/agpl-3.0.html).
