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

/-- Version-two tile predictor domain. -/
inductive PredictionMode
  | paeth
  | temporal
  | average
  | clampGradient
  | halfGradient
  deriving DecidableEq, Repr

/-- Version-two prediction-mode byte mapping. -/
def predictionModeCode : PredictionMode → Nat
  | .paeth => 0
  | .temporal => 1
  | .average => 2
  | .clampGradient => 3
  | .halfGradient => 4

theorem prediction_mode_code_bound (mode : PredictionMode) :
    predictionModeCode mode ≤ 4 := by
  cases mode <;> decide

/-- Integer average used by the version-two average predictor. -/
def average2 (left above : Nat) : Nat := (left + above) / 2

theorem average2_bound (left above maximum : Nat)
    (hl : left ≤ maximum) (ha : above ≤ maximum) :
    average2 left above ≤ maximum := by
  simp only [average2]
  omega

/-- Clamp a signed prediction to the unsigned sample interval. -/
def clampSample (prediction : Int) (maximum : Nat) : Nat :=
  if prediction < 0 then 0 else min prediction.toNat maximum

theorem clamp_sample_bound (prediction : Int) (maximum : Nat) :
    clampSample prediction maximum ≤ maximum := by
  simp only [clampSample]
  split
  · omega
  · exact Nat.min_le_right _ _

end Fastvid
