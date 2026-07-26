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

/-- The mask modulus for a fixed-width unsigned block symbol. -/
def fixedWidthModulus (width : Nat) : Nat := 2 ^ width

/-- A value that fits the signaled block width survives fixed-width packing. -/
theorem fixed_width_roundtrip (value width : Nat)
    (fits : value < fixedWidthModulus width) :
    value % fixedWidthModulus width = value := by
  exact Nat.mod_eq_of_lt fits

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

/-- Version-three rANS table size. -/
def ransTableSize (tableLog : Nat) : Nat := 2 ^ tableLog

/-- Slot selected from the low `tableLog` bits of an rANS state. -/
def ransSlot (state tableLog : Nat) : Nat :=
  state % ransTableSize tableLog

/-- Every decoded slot lies inside its normalized frequency table. -/
theorem rans_slot_bound (state tableLog : Nat) :
    ransSlot state tableLog < ransTableSize tableLog := by
  simp only [ransSlot, ransTableSize]
  exact Nat.mod_lt state (Nat.two_pow_pos tableLog)

/-- Interleaved mode assigns each raster sample to a cyclic rANS state. -/
def ransStateIndex (sampleIndex stateCount : Nat) : Nat :=
  sampleIndex % stateCount

/-- A cyclic rANS state index is valid whenever the format has a state. -/
theorem rans_state_index_bound
    (sampleIndex stateCount : Nat) (h : 0 < stateCount) :
    ransStateIndex sampleIndex stateCount < stateCount := by
  simp only [ransStateIndex]
  exact Nat.mod_lt sampleIndex h

/-- The inverse rANS state for a slot in a symbol's frequency interval. -/
def ransDecodeState
    (state frequency cumulative tableLog : Nat) : Nat :=
  frequency * (state / ransTableSize tableLog)
    + ransSlot state tableLog - cumulative

/-- A normalized symbol interval always has a nonnegative slot offset. -/
theorem rans_slot_offset_nonnegative
    (state cumulative tableLog : Nat)
    (h : cumulative ≤ ransSlot state tableLog) :
    cumulative + (ransSlot state tableLog - cumulative) =
      ransSlot state tableLog := by
  omega

/-- Version-four rows per independently reconstructed predictor band. -/
def parallelBandRows : Nat := 64

/-- Version-four folded residuals per independently delimited entropy shard. -/
def parallelShardSymbols : Nat := 4096

/-- Maximum number of byte-aligned Rice lanes in one entropy shard. -/
def parallelRiceLanes : Nat := 4

/-- Implicit predictor-band index for a tile-local row. -/
def predictorBandIndex (row : Nat) : Nat := row / parallelBandRows

/-- Row position inside its implicit predictor band. -/
def predictorBandRow (row : Nat) : Nat := row % parallelBandRows

/-- The implicit band index and row position reconstruct the tile row. -/
theorem predictor_band_recompose (row : Nat) :
    predictorBandIndex row * parallelBandRows + predictorBandRow row = row := by
  unfold predictorBandIndex predictorBandRow parallelBandRows
  rw [Nat.mul_comm]
  exact Nat.div_add_mod row 64

/-- Every implicit predictor-band row lies inside the 64-row bound. -/
theorem predictor_band_row_bound (row : Nat) :
    predictorBandRow row < parallelBandRows := by
  simpa [predictorBandRow, parallelBandRows] using
    Nat.mod_lt row (by decide : 0 < 64)

/-- Round-robin assignment of a shard-local symbol to a Rice lane. -/
def riceLaneIndex (symbol laneCount : Nat) : Nat := symbol % laneCount

/-- Every Rice symbol selects an existing lane. -/
theorem rice_lane_index_bound (symbol laneCount : Nat) (h : 0 < laneCount) :
    riceLaneIndex symbol laneCount < laneCount := by
  simp [riceLaneIndex]
  exact Nat.mod_lt symbol h

/-- A default-width version-four band contains at most 16,384 samples. -/
theorem default_predictor_band_sample_bound
    (width rows : Nat) (hw : width ≤ 256) (hr : rows ≤ parallelBandRows) :
    width * rows ≤ 16384 := by
  calc
    width * rows ≤ 256 * 64 := Nat.mul_le_mul hw (by simpa [parallelBandRows] using hr)
    _ = 16384 := by decide

/-- Four lanes bound a full 4,096-symbol shard to 1,024 symbols per lane. -/
theorem full_shard_lane_span :
    parallelShardSymbols / parallelRiceLanes = 1024 := by
  decide

end Fastvid
