namespace Fastvid

/-- Map a signed residual to the nonnegative zigzag domain. -/
def zigzag : Int → Nat
  | Int.ofNat n => 2 * n
  | Int.negSucc n => 2 * n + 1

/-- Inverse of `zigzag`. -/
def unzigzag (n : Nat) : Int :=
  if n % 2 = 0 then Int.ofNat (n / 2) else Int.negSucc (n / 2)

theorem unzigzag_zigzag (x : Int) : unzigzag (zigzag x) = x := by
  cases x with
  | ofNat n =>
      simp [zigzag, unzigzag]
  | negSucc n =>
      simp [zigzag, unzigzag, Nat.add_mod, Nat.add_div]

/-- The power-of-two divisor selected by Rice parameter `k`. -/
def riceDivisor (k : Nat) : Nat := 2 ^ k

/-- Unary-coded high part of a Rice-coded nonnegative value. -/
def riceQuotient (n k : Nat) : Nat := n / riceDivisor k

/-- Fixed-width low part of a Rice-coded nonnegative value. -/
def riceRemainder (n k : Nat) : Nat := n % riceDivisor k

/-- Rice quotient and remainder reconstruct the original folded residual. -/
theorem rice_recompose (n k : Nat) :
    riceQuotient n k * riceDivisor k + riceRemainder n k = n := by
  simp [riceQuotient, riceRemainder, riceDivisor, Nat.mul_comm, Nat.div_add_mod]

end Fastvid
