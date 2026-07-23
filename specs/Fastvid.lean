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

end Fastvid
