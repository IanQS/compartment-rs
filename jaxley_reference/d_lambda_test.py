"""d_lambda verification

Run `python jaxley_reference/d_lambda_test.py` from the top-level
"""

import jaxley as jx
import numpy as np
from loguru import logger

SWC_PATH = "data/morph_minimal.swc"
logger.info(f"SWC Path: {SWC_PATH}")


def inspect_branches(swc_path: str) -> None:
    """To make sure that we're even parsing the branches the same way as Jaxley, we:

    1) go through the number of branches
    2) for each branch, print the various properties
    """
    cell = jx.read_swc(swc_path, ncomp=1)

    logger.info(f"Number of branches: {len(list(cell.branches))}")
    for branch in cell.branches:
        n = branch.nodes
        assert n is not None
        keys = ["radius", "length", "capacitance", "axial_resistivity"]
        accum = {k: n[k].to_numpy()[0] for k in keys}
        logger.info(accum)


def d_lambda_ncomp(swc_path: str, d_lambda: float, frequency: float = 100.0) -> list:
    """Compute per-branch ncomp using the d_lambda rule, matching Jaxley's approach.

    Lifted directly from: https://jaxley.readthedocs.io/en/latest/how_to_guide/set_ncomp.html
    """
    cell = jx.read_swc(swc_path, ncomp=1)
    ncomps = []

    for branch in cell.branches:
        diameter = 2 * branch.nodes["radius"].to_numpy()[0]
        c_m = branch.nodes["capacitance"].to_numpy()[0]
        r_a = branch.nodes["axial_resistivity"].to_numpy()[0]
        l = branch.nodes["length"].to_numpy()[0]

        lambda_f = 1e5 * np.sqrt(diameter / (4 * np.pi * frequency * c_m * r_a))
        ncomp = int((l / (d_lambda * lambda_f) + 0.9) / 2) * 2 + 1
        ncomps.append(ncomp)
        branch.set_ncomp(ncomp, initialize=False)

    cell.initialize()
    return ncomps


if __name__ == "__main__":
    print("1. Branch properties from Jaxley")
    inspect_branches(SWC_PATH)

    print("2. d_lambda rule results")
    for d_lambda in [0.1, 0.01]:
        ncomps = d_lambda_ncomp(SWC_PATH, d_lambda)
        total = sum(ncomps)
        logger.info({"d_lambda": d_lambda, "per-branch comps": ncomps, "total compartments": total})
