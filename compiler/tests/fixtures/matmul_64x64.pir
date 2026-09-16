# PIR Golden Fixture: matmul_64x64
# Matrix multiplication C[i][j] += A[i][k] * B[k][j]
# Affine bounds: 0 <= i < 64, 0 <= j < 64, 0 <= k < 64

[parameters]
N = 64
M = 64
K = 64

[domain matmul_domain]
dims = 3
n_iter = 3
n_param = 0
constraints = [
  [1, 0, 0, 0],      # i >= 0
  [-1, 0, 0, 63],    # i <= 63
  [0, 1, 0, 0],      # j >= 0
  [0, -1, 0, 63],    # j <= 63
  [0, 0, 1, 0],      # k >= 0
  [0, 0, -1, 63],    # k <= 63
]

[schedule_tree]
root = Band {
  members = [
    { matrix = [[1, 0, 0]], constant = [0] },    # i loop
  ]
  coincident = [false]
  child = Band {
    members = [
      { matrix = [[0, 1, 0]], constant = [0] },  # j loop
    ]
    coincident = [true]
    child = Band {
      members = [
        { matrix = [[0, 0, 1]], constant = [0] },  # k loop
      ]
      coincident = [true]
      child = Domain {
        stmt_id = S0
        domain = matmul_domain
      }
    }
  }
}

[statements]
S0 = {
  domain = matmul_domain
  body = "C[i][j] += A[i][k] * B[k][j]"
  quantity = Many
  mutability = Immutable
}

[accesses]
S0_read_A = {
  stmt_id = S0
  stmt_domain = matmul_domain
  map = { pieces = [{ domain = matmul_domain, matrix = [[1, 0, 0]], constant = [0] }] }
  access_type = Read
  array_name = "A"
}
S0_read_B = {
  stmt_id = S0
  stmt_domain = matmul_domain
  map = { pieces = [{ domain = matmul_domain, matrix = [[0, 0, 1]], constant = [0] }] }
  access_type = Read
  array_name = "B"
}
S0_write_C = {
  stmt_id = S0
  stmt_domain = matmul_domain
  map = { pieces = [{ domain = matmul_domain, matrix = [[1, 0, 0], [0, 1, 0]], constant = [0, 0] }] }
  access_type = Write
  array_name = "C"
}

[quantities]
N = Zero
M = Zero
K = Zero