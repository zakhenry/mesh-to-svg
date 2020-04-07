# Mesh to SVG

WASM Library (written in Rust 🦀) to convert meshes (optionally with supplemental wireframe mesh) into an SVG line drawing

It is recommended to use https://github.com/zakhenry/svg-from-wireframe which wraps this library in a more ergonomic interface,
and provides demos for how to integrate with webworkers in Angular.

## Installation

```sh
$ yarn add mesh-to-svg
```

## Contributing

### Setup

Clone this repo
```console
$ git clone git@github.com:zakhenry/mesh-to-svg.git
````

Ensure you have a rust toolchain set up, if not:

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install [wasm-pack](https://rustwasm.github.io/docs/wasm-pack/) (the tooling for making WASM modules with Rust)

```console
$ curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Fetch deps (this can be skipped as wasm-pack will do this too)

```console
$ cargo build
```

### Build

```console
$ wasm-pack build -- --features console_log
```

*or* use the handy yarn helper if you're more familiar with frontend tooling

```console
$ yarn wasm:build:debug
```

To do a release build (way faster to run, but runtime errors are less helpful)

```console
$ wasm-pack build --release -- --features console_log
```

*or* use the handy yarn helper if you're more familiar with frontend tooling

```console
$ yarn wasm:build:release
```


### Link package

Link npm package so other repos can use this module

```console
$ cd pkg && yarn link
```

Link from your other package

```console
$ cd path/to/your/other/repo
$ yarn link mesh-to-svg
```

You only have to do this once (it sets up symlinks)

Now you can just rebuild this project, and if you have a file watcher on the other project (the default for Angular for example)
when you rebuild the wasm binary the webpage will automatically reload with the latest code.

## Releasing a package

Package releases are all done automatically by [`semantic-release`](https://github.com/semantic-release/semantic-release) and TravisCI.

Please follow the [semantic commit guidelines](https://github.com/semantic-release/semantic-release#commit-message-format) so your commit messages will automatically generate the changelog and the correct semver.

## Issues & PRs

Please raise issues for features you'd like to see, issues encountered etc. PRs are _always_ welcome, I really want to learn how to make this package better, faster, stronger!

## Tips
Calling `log!()` is super slow. Any performance benchmarks you may do with logging will be seriously polluted by the calls out to JS.

As such `console_log` is an optional feature, which is turned off for the release build

Run a build with the feature `console_log` turned off:

wasm-pack build --release

or

```console
$ yarn wasm:build:prod
```


