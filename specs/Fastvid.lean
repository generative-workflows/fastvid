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

/-- Signed residual from a co-located reconstructed reference sample. -/
def temporalResidual (current previous : Int) : Int := current - previous

/-- Adding a temporal residual to its reference reconstructs the sample. -/
theorem temporal_recompose (current previous : Int) :
    previous + temporalResidual current previous = current := by
  simp only [temporalResidual]
  omega

/-- Largest unsigned sample represented by `bitDepth` bits. -/
def sampleMax (bitDepth : Nat) : Nat := 2 ^ bitDepth - 1

/-- Version-one quantizer scaling, stated without implementation-width details. -/
def quantStep (quality bitDepth : Nat) : Nat :=
  1 + ((100 - quality) / 5) * 2 ^ (bitDepth - 8)

/-- Quality 100 is lossless at every bit depth because its step is one. -/
theorem quant_step_quality_100 (bitDepth : Nat) :
    quantStep 100 bitDepth = 1 := by
  simp [quantStep]

/-- A nonnegative in-range residual folds below twice the sample maximum. -/
theorem zigzag_positive_bound (residual maximum : Nat)
    (h : residual ≤ maximum) :
    zigzag (Int.ofNat residual) ≤ 2 * maximum := by
  simp [zigzag]
  omega

/-- A negative residual of magnitude at most `maximum` has the same bound. -/
theorem zigzag_negative_bound (predecessor maximum : Nat)
    (h : predecessor < maximum) :
    zigzag (Int.negSucc predecessor) ≤ 2 * maximum := by
  simp [zigzag]
  omega

theorem max_folded_10 : 2 * sampleMax 10 = 2046 := by
  decide

theorem max_folded_12 : 2 * sampleMax 12 = 8190 := by
  decide

theorem max_folded_16 : 2 * sampleMax 16 = 131070 := by
  decide

end Fastvid
