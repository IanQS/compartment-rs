- [compartment-rs](#compartment-rs)
  - [Motivation](#motivation)
  - [Features](#features)
  - [SWC Convention](#swc-convention)

# compartment-rs

A simple Neuroscience compartment modeling library, written in Rust with Python bindings. Aims to keep circuits "local" instead of global, which should, in theory, allow multiple simulations within a single process.

## Motivation

I don't know as much as I should about neuroscience computational models. This is my way of trying to force myself to build something to learn, instead of just reading a textbook and not applying my knowledge.

## Features

- [x] `.swc` reader that topologically sorts the input `.swc` file and warns for 0-radius components.

- [ ] constructs compartment models via a multi-linked list.

- [ ] Will support `d-lambda` rule as outlined in the [NEURON Book - Chapter 5](https://www.fuw.edu.pl/~suffa/Modelowanie/NEURON%20-%20Book/chap5.pdf), page 28, under `d-lambda` rule
  - Will take an existing multi-linked list and "resize" it

- [ ] Hodgkin-Huxley Dynamics

## SWC Convention

We use the convention set out by [Neuronland](http://www.neuronland.org/NLMorphologyConverter/MorphologyFormats/SWC/Spec.html), which seems to be the canonical one
