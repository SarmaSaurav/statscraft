"""Reference values for the GLM tests, computed with statsmodels.

The statsmodels `f_test` gives the Type III sums of squares, which is
what StatsCraft's GLM reports. The numbers printed here are pasted
into the Rust test as the expected values.

Run with:  C:/Users/Saurav/miniforge3/python.exe tools/glm_reference.py
"""
import numpy as np
from statsmodels.formula.api import ols


def report(title, a, b, y):
    frame = {"y": y, "a": a, "b": b}
    model = ols("y ~ C(a)*C(b)", data=frame).fit()
    print(f"=== {title} ===")
    print(f"  n = {len(y)}   residual SS = {model.ssr:.10f}   df = {model.df_resid}")
    # The Type III sums of squares, straight from the anova table.
    from statsmodels.stats.anova import anova_lm

    table = anova_lm(model, typ=3)
    print(table.to_string())
    print()
    for effect in ["C(a)", "C(b)", "C(a):C(b)"]:
        row = table.loc[effect]
        print(
            f"  {effect:12} ss={row['sum_sq']:.10f} df={row['df']:.1f} "
            f"F={row['F']:.10f} p={row['PR(>F)']:.6e}"
        )
    print()
    return model


def main():
    # The same deterministic data the Rust test uses.
    rng = np.random.default_rng(20240501)
    n = 90
    a = rng.integers(0, 3, n)
    b = rng.integers(0, 2, n)
    y = 10 + 2.0 * a + 1.5 * b + 0.5 * a * b + rng.normal(0, 3.0, n)
    report("the unbalanced two-factor design", a, b, y)

    # Write the data out so the Rust test uses exactly the same cases.
    import os

    out = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "samples",
        "glm_reference.csv",
    )
    with open(out, "w", encoding="utf-8", newline="") as f:
        f.write("a,b,y\n")
        for i in range(n):
            f.write(f"{a[i]},{b[i]},{float(y[i])!r}\n")
    print(f"the data: {out}")


if __name__ == "__main__":
    main()
