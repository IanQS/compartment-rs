- [compartment-rs](#compartment-rs)
  - [Motivation](#motivation)
  - [Features](#features)
  - [SWC Convention](#swc-convention)
  - [Python Development](#python-development)
    - [Dev tooling](#dev-tooling)
    - [Running tests](#running-tests)
    - [Validation against Jaxley](#validation-against-jaxley)

# compartment-rs

A simple Neuroscience compartment modeling library, written in Rust with Python bindings. Aims to keep circuits "local" instead of global, which should, in theory, allow multiple simulations within a single process.

## Resources:

### Neuroscience

[mrgreene09 - computational neuroscience textbook (Hodgkin Huxley Chapter)](https://mrgreene09.github.io/computational-neuroscience-textbook/Ch4.html)

- great quick reference

[Dayan and Abbott - Theoretical Neuroscience](https://boulderschool.yale.edu/sites/default/files/files/DayanAbbott.pdf)

- more in-depth information

[Neuronal Dynamics](https://neuronaldynamics.epfl.ch/)

- I'd say around the level of Dayan and Abbott, even if it seems more approachable; this book seems more like a resources to get grad students started rather than a deep-dive into the various topics

## Motivation

I don't know as much as I should about neuroscience computational models. This is my way of trying to force myself to build something to learn, instead of just reading a textbook and not applying my knowledge.

## Features

See [Plan](./plan.md) for a rough outline of features. This will be updated over time

## SWC Convention

We use the convention set out by [Neuronland](http://www.neuronland.org/NLMorphologyConverter/MorphologyFormats/SWC/Spec.html), which seems to be the canonical one

## Python Development

Python **development** is managed via `uv` and is **mostly** just tests for ensuring correctness (see [Validation against Jaxley](#validation-against-jaxley)) and ensuring our rust code is performing correctly.

If we build this right, python will just be a shim layer

### Dev tooling

```sh
uv sync --group dev
```

### Running tests

Note: we dob't have any tests right now in python. We're just going off vibes and outputting the jaxley results to the screen

```sh
uv sync --group test
pytest
```

### Validation against Jaxley

The `validation` group installs [Jaxley](https://jaxley.readthedocs.io/), which requires Python >=3.10 and pulls in JAX. It is intentionally kept separate since it is not needed at runtime.

```sh
uv sync --group validation
```

Validation scripts live in [jaxley_reference](./jaxley_reference).
