# PIR Golden Fixture: teleport
# Quantum teleportation protocol
# Alice has |ψ⟩ = α|0⟩ + β|1⟩, shares Bell pair |Φ⁺⟩ with Bob
# Alice measures in Bell basis, sends 2 classical bits to Bob
# Bob applies X^b1 Z^b2 to recover |ψ⟩

[parameters]
N = 3  # 3 qubits: ψ, Alice's Bell, Bob's Bell

[domain teleport_domain]
dims = 0
n_iter = 0
n_param = 0
constraints = []

[schedule_tree]
root = Sequence {
  children = [
    # Prepare Bell pair |Φ⁺⟩ = (|00⟩ + |11⟩)/√2
    Domain {
      stmt_id = S_prepare_bell
      domain = teleport_domain
    },
    # Alice's CNOT and H on her qubit
    Domain {
      stmt_id = S_alice_ops
      domain = teleport_domain
    },
    # Alice measures in Bell basis (2 classical bits)
    Domain {
      stmt_id = S_alice_measure
      domain = teleport_domain
    },
    # Bob applies corrections based on measurement results
    Domain {
      stmt_id = S_bob_corrections
      domain = teleport_domain
    }
  ]
}

[statements]
S_prepare_bell = {
  domain = teleport_domain
  body = "H q[1]; CNOT q[1], q[2]"
  quantity = One
  mutability = Mutable
}
S_alice_ops = {
  domain = teleport_domain
  body = "CNOT q[0], q[1]; H q[0]"
  quantity = One
  mutability = Mutable
}
S_alice_measure = {
  domain = teleport_domain
  body = "b0 = measure q[0]; b1 = measure q[1]"
  quantity = Zero
  mutability = Immutable
}
S_bob_corrections = {
  domain = teleport_domain
  body = "if b1 { X q[2] }; if b0 { Z q[2] }"
  quantity = One
  mutability = Mutable
}

[accesses]
S_prepare_bell_q1 = {
  stmt_id = S_prepare_bell
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [1] }] }
  access_type = ReadWrite
  array_name = "q"
}
S_prepare_bell_q2 = {
  stmt_id = S_prepare_bell
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [2] }] }
  access_type = ReadWrite
  array_name = "q"
}
S_alice_ops_q0 = {
  stmt_id = S_alice_ops
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [0] }] }
  access_type = ReadWrite
  array_name = "q"
}
S_alice_ops_q1 = {
  stmt_id = S_alice_ops
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [1] }] }
  access_type = ReadWrite
  array_name = "q"
}
S_alice_measure_q0 = {
  stmt_id = S_alice_measure
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [0] }] }
  access_type = Read
  array_name = "q"
}
S_alice_measure_q1 = {
  stmt_id = S_alice_measure
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [1] }] }
  access_type = Read
  array_name = "q"
}
S_bob_corrections_q2 = {
  stmt_id = S_bob_corrections
  stmt_domain = teleport_domain
  map = { pieces = [{ domain = teleport_domain, matrix = [[]], constant = [2] }] }
  access_type = ReadWrite
  array_name = "q"
}

[quantities]
q[0] = One
q[1] = One
q[2] = One
b0 = Zero
b1 = Zero