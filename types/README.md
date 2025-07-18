# Types

Lua API documentation for `pattrns` for the [LuaLS](https://luals.github.io/) Lua Language server.

You can also read the API docs online in the [API reference](https://renoise.github.io/pattrns/API/index.html) chapter of the `pattrns` book.


## Installation

If you want to use the type definitions directly for autocompletion, and to view the API documentation in e.g. vscode and other editors that support the LuaLS language server:

- First install the [sumneko.lua vscode extension](https://luals.github.io/#vscode-install).
- Then download a copy of the [`./types/pattrns`](./pattrns) type definitions folder and configure your workspace to use the files in your project. To do this, add the following to your project's `/.vscode/settings.json` file

```json
{
    "Lua.workspace.library": ["PATH/TO/PATTRNS_TYPES_FOLDER"]
}
```

For configuring other editors, you can check out the [official docs about installation](https://luals.github.io/#install)


## License

`pattrns` is distributed under the terms of the [GNU Affero General Public License V3](https://www.gnu.org/licenses/agpl-3.0.html).
