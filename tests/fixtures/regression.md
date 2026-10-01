# Regression

Consider the linear model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The OLS estimator is

$$
\hat\beta =
\left(
\sum_{i=1}^{n} X_i X_i^\top
\right)^{-1}
\sum_{i=1}^{n} X_i Y_i.
$$

Under

$$
\mathbb E[\varepsilon_i \mid X_i] = 0,
$$

the estimator is **consistent**.

## Notes

- $X_i \in \mathbb R^K$.
- $\beta$ is a $K \times 1$ vector.
- Standard errors can be *heteroskedasticity robust*.

```rust
fn estimate(x: Matrix, y: Vector) -> Vector {
    solve(x.t() * x, x.t() * y)
}
```
