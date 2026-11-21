/-!
# Strict JSON reader

The checker reads certificates with its own JSON reader instead of Lean's
`Lean.Json.parse`. The built-in parser keeps only the last of two repeated
keys and normalises numbers such as `1e0`, `1.0` and `-0` to `1` and `0`, so
it would accept certificates that the certificate format forbids and that the
engine's reader rejects.

This reader accepts the subset of JSON that certificates use: objects, arrays,
strings and non-negative integers written as plain decimal digits. Objects
keep every key in order, including repeats, so later validation can reject
duplicates. `true`, `false`, `null`, signs, fractions and exponents are
rejected. Every recursive call either consumes input or descends one level,
and both are bounded, so the reader always terminates. Objects are limited to
`maxMembers` members and arrays to `maxItems` items, which bounds the
interpreter's recursion depth as well as the work done by later validation.
-/

namespace AxiomCheck.Json

/-- A JSON value in the subset that certificates use. -/
inductive Value where
  /-- A string, with escape sequences decoded. -/
  | str (text : String)
  /-- A non-negative integer and the number of decimal digits it was written
  with. -/
  | nat (value : Nat) (digits : Nat)
  /-- An array, in order. -/
  | arr (items : List Value)
  /-- An object's members, in order, with repeated keys kept. -/
  | obj (members : List (String × Value))
  deriving Inhabited

/-- Why the text is not JSON in the accepted subset. The message never
contains text copied from the input. -/
structure Error where
  /-- The zero-based character position where reading stopped. -/
  position : Nat
  /-- What was wrong at that position. -/
  reason : String
  deriving Repr

/-- The deepest nesting of arrays and objects the reader accepts. A
certificate needs four levels. -/
def maxDepth : Nat := 8

/-- The most members an object may have. A certificate's largest object has
four keys. -/
def maxMembers : Nat := 8

/-- The most items an array may have. A certificate has at most 16 rows of 16
entries, and at most 16 distinct claims. -/
def maxItems : Nat := 64

/-- The longest run of digits accepted in an integer. Large enough for any
64-bit unsigned value. -/
def maxIntegerDigits : Nat := 20

/-- Whether `c` is JSON whitespace. -/
def isWhitespace (c : Char) : Bool :=
  c == ' ' || c == '\t' || c == '\n' || c == '\r'

/-- The position of the first character at or after `i` that is not
whitespace. -/
def skipWhitespace (input : Array Char) (i : Nat) : Nat :=
  go (input.size - i) i
where
  /-- Skips whitespace, using `fuel` to bound the number of steps. -/
  go : Nat → Nat → Nat
    | 0, j => j
    | fuel + 1, j =>
      match input[j]? with
      | some c => if isWhitespace c then go fuel (j + 1) else j
      | none => j

/-- The value of a hexadecimal digit. -/
def hexValue (c : Char) : Option Nat :=
  if '0' ≤ c && c ≤ '9' then some (c.toNat - '0'.toNat)
  else if 'a' ≤ c && c ≤ 'f' then some (c.toNat - 'a'.toNat + 10)
  else if 'A' ≤ c && c ≤ 'F' then some (c.toNat - 'A'.toNat + 10)
  else none

/-- The code unit written as four hexadecimal digits starting at `i`. -/
def readHex4 (input : Array Char) (i : Nat) : Option Nat := do
  let a ← hexValue (← input[i]?)
  let b ← hexValue (← input[i + 1]?)
  let c ← hexValue (← input[i + 2]?)
  let d ← hexValue (← input[i + 3]?)
  pure (((a * 16 + b) * 16 + c) * 16 + d)

/-- Decodes a `\u` escape whose four hex digits start at `i`, joining a
surrogate pair when one follows. Returns the character and the position after
the escape. Lone surrogates are rejected. -/
def readUnicodeEscape (input : Array Char) (i : Nat) : Except Error (Char × Nat) := do
  let some unit := readHex4 input i
    | throw ⟨i, "invalid unicode escape"⟩
  if 0xD800 ≤ unit && unit ≤ 0xDBFF then
    let isLow := input[i + 4]? == some '\\' && input[i + 5]? == some 'u'
    let some low := (if isLow then readHex4 input (i + 6) else none)
      | throw ⟨i, "unpaired surrogate"⟩
    if 0xDC00 ≤ low && low ≤ 0xDFFF then
      let code := 0x10000 + (unit - 0xD800) * 0x400 + (low - 0xDC00)
      pure (Char.ofNat code, i + 10)
    else
      throw ⟨i, "unpaired surrogate"⟩
  else if 0xDC00 ≤ unit && unit ≤ 0xDFFF then
    throw ⟨i, "unpaired surrogate"⟩
  else
    pure (Char.ofNat unit, i + 4)

/-- Reads the body of a string whose opening quote was at `i - 1`, returning
the decoded text and the position after the closing quote. -/
def readString (input : Array Char) (i : Nat) : Except Error (String × Nat) :=
  go (input.size - i + 1) i ""
where
  /-- Reads characters into `acc`, using `fuel` to bound the number of
  steps. -/
  go : Nat → Nat → String → Except Error (String × Nat)
    | 0, j, _ => throw ⟨j, "unterminated string"⟩
    | fuel + 1, j, acc =>
      match input[j]? with
      | none => throw ⟨j, "unterminated string"⟩
      | some '"' => pure (acc, j + 1)
      | some '\\' =>
        match input[j + 1]? with
        | some '"' => go fuel (j + 2) (acc.push '"')
        | some '\\' => go fuel (j + 2) (acc.push '\\')
        | some '/' => go fuel (j + 2) (acc.push '/')
        | some 'b' => go fuel (j + 2) (acc.push (Char.ofNat 8))
        | some 'f' => go fuel (j + 2) (acc.push (Char.ofNat 12))
        | some 'n' => go fuel (j + 2) (acc.push '\n')
        | some 'r' => go fuel (j + 2) (acc.push '\r')
        | some 't' => go fuel (j + 2) (acc.push '\t')
        | some 'u' =>
          match readUnicodeEscape input (j + 2) with
          | .ok (c, next) => go fuel next (acc.push c)
          | .error e => .error e
        | _ => throw ⟨j, "invalid escape"⟩
      | some c =>
        if c.toNat < 0x20 then throw ⟨j, "control character in string"⟩
        else go fuel (j + 1) (acc.push c)

/-- Reads a non-negative integer of plain decimal digits starting at `i`. A
leading zero is allowed only for `0` itself, and a sign, fraction or exponent
is rejected. -/
def readInteger (input : Array Char) (i : Nat) : Except Error (Value × Nat) :=
  let digits := go (input.size - i) i []
  let count := digits.length
  if count == 0 then throw ⟨i, "expected a value"⟩
  else if count > maxIntegerDigits then throw ⟨i, "integer has too many digits"⟩
  else if count > 1 && digits.head? == some '0' then throw ⟨i, "leading zero"⟩
  else
    let next := i + count
    match input[next]? with
    | some '.' | some 'e' | some 'E' => throw ⟨next, "only plain integers are allowed"⟩
    | _ =>
      let value := digits.foldl (fun total d => total * 10 + (d.toNat - '0'.toNat)) 0
      pure (.nat value count, next)
where
  /-- Collects consecutive digits, using `fuel` to bound the number of
  steps. -/
  go : Nat → Nat → List Char → List Char
    | 0, _, acc => acc.reverse
    | fuel + 1, j, acc =>
      match input[j]? with
      | some c => if c.isDigit then go fuel (j + 1) (c :: acc) else acc.reverse
      | none => acc.reverse

mutual
  /-- Reads one value at `i`, at nesting level `depth`. -/
  def readValue (input : Array Char) : Nat → Nat → Nat → Except Error (Value × Nat)
    | 0, _, i => throw ⟨i, "input too complex"⟩
    | fuel + 1, depth, i =>
      let i := skipWhitespace input i
      match input[i]? with
      | some '"' => do
        let (text, next) ← readString input (i + 1)
        pure (.str text, next)
      | some '[' =>
        if depth ≥ maxDepth then throw ⟨i, "nested too deeply"⟩
        else
          let j := skipWhitespace input (i + 1)
          if input[j]? == some ']' then pure (.arr [], j + 1)
          else readItems input fuel (depth + 1) j []
      | some '{' =>
        if depth ≥ maxDepth then throw ⟨i, "nested too deeply"⟩
        else
          let j := skipWhitespace input (i + 1)
          if input[j]? == some '}' then pure (.obj [], j + 1)
          else readMembers input fuel (depth + 1) j []
      | some c =>
        if c.isDigit then readInteger input i
        else throw ⟨i, "unexpected character"⟩
      | none => throw ⟨i, "unexpected end of input"⟩

  /-- Reads the items of an array after its opening bracket, collecting them
  in reverse in `acc`. -/
  def readItems (input : Array Char) :
      Nat → Nat → Nat → List Value → Except Error (Value × Nat)
    | 0, _, i, _ => throw ⟨i, "input too complex"⟩
    | fuel + 1, depth, i, acc => do
      let (item, next) ← readValue input fuel depth i
      let next := skipWhitespace input next
      match input[next]? with
      | some ',' =>
        if acc.length + 1 ≥ maxItems then throw ⟨next, "array has too many items"⟩
        else readItems input fuel depth (next + 1) (item :: acc)
      | some ']' => pure (.arr (item :: acc).reverse, next + 1)
      | _ => throw ⟨next, "expected , or ] in array"⟩

  /-- Reads the members of an object after its opening brace, collecting them
  in reverse in `acc`. -/
  def readMembers (input : Array Char) :
      Nat → Nat → Nat → List (String × Value) → Except Error (Value × Nat)
    | 0, _, i, _ => throw ⟨i, "input too complex"⟩
    | fuel + 1, depth, i, acc => do
      let i := skipWhitespace input i
      if input[i]? != some '"' then throw ⟨i, "expected a key"⟩
      let (key, next) ← readString input (i + 1)
      let next := skipWhitespace input next
      if input[next]? != some ':' then throw ⟨next, "expected : after key"⟩
      let (value, next) ← readValue input fuel depth (next + 1)
      let next := skipWhitespace input next
      match input[next]? with
      | some ',' =>
        if acc.length + 1 ≥ maxMembers then throw ⟨next, "object has too many members"⟩
        else readMembers input fuel depth (next + 1) ((key, value) :: acc)
      | some '}' => pure (.obj ((key, value) :: acc).reverse, next + 1)
      | _ => throw ⟨next, "expected , or } in object"⟩
end

/-- Reads `text` as a single JSON value with nothing but whitespace after it.
A byte order mark is rejected as an unexpected character. -/
def parse (text : String) : Except Error Value := do
  let input := text.toList.toArray
  let (value, next) ← readValue input (2 * input.size + 2) 0 0
  let next := skipWhitespace input next
  if next < input.size then throw ⟨next, "unexpected data after the value"⟩
  pure value

end AxiomCheck.Json
