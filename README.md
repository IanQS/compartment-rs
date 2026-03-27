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

- [x] `.swc` reader that topologically sorts the input `.swc` file and warns for 0-radius components.

- [x] constructs compartment models via a multi-linked list.

- [ ] Implement branching, where we coalesce multiple compartment models into higher-level "branches"
  - [x] Will support `d-lambda` rule as outlined in the [NEURON Book - Chapter 5](https://www.fuw.edu.pl/~suffa/Modelowanie/NEURON%20-%20Book/chap5.pdf), page 28, under `d-lambda` rule
  - Will take an existing multi-linked list and "resize" it

- [ ] Dynamics
  - [ ] Hodgkin-Huxley Dynamics
  - [ ] LIF Dynamics
  - [ ] Passive Membrane Potential

- [ ] Synapse Dynamics
  - [ ] Exp2Syn

- [ ] Applying Currents

## SWC Convention

We use the convention set out by [Neuronland](http://www.neuronland.org/NLMorphologyConverter/MorphologyFormats/SWC/Spec.html), which seems to be the canonical one


## Python Development

Python **development** is managed via `uv` and is mostly just tests for ensuring correctness both as a sanity check (see [Validation against Jaxley](#validation-against-jaxley)) and to check that our built rust code is performing correctly. 

If we build this right, python will just be a shim layer

### Dev tooling

Includes `ruff` (linting/formatting) and `basedpyright` (type checking):

```sh
uv sync --group dev
```

### Running tests

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
