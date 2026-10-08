# Strata

A catalogue for large image folders.

A desktop app for macOS. Strata takes in folders of images and reads what each
picture holds: its metadata, its colours, and the prompt behind it when Draw
Things made it. Search, sort and gather from there.

---

## Use it

There is no download. Build it from source, below.

Add a folder with **Add** and choose which of its folders to take in. Each
import shows up in the left rail as a batch, beside the library as a whole.

Search runs over file names, keywords, captions and Draw Things prompts. A
search can be saved to the rail. Favourites, colour labels and collections
gather images without copying them. Most changes can be undone with `Cmd-Z`.

**The library**
Strata copies each image into its library, `~/Pictures/Strata Library.strata`,
along with everything it reads from it. The originals stay where they were.
Switch off **Leave source files in place** in Settings (`Cmd-,`) and they go to
the Trash after each import instead, so the library holds the only copy.

---

## Build it

**Prerequisites**
[Rust](https://rustup.rs), [Node](https://nodejs.org),
[pnpm](https://pnpm.io), [just](https://github.com/casey/just), and the Xcode
Command Line Tools (`xcode-select --install`).
`just check` also needs
[preset-compliance](https://github.com/preset-nz/compliance)
(`cargo binstall preset-compliance`). `just prep` reports what is installed.

```sh
just install     # dependencies
just run         # the app
```

`just build` makes a release bundle. `just check` runs the tests, linters and
licence gate.

---

## Licence

[MIT](LICENSE).
