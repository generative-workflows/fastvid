#!/usr/bin/awk -f

function sort_numeric(values, count,    i, j, item) {
  for (i = 2; i <= count; i++) {
    item = values[i]
    j = i - 1
    while (j >= 1 && values[j] > item) {
      values[j + 1] = values[j]
      j--
    }
    values[j + 1] = item
  }
}

function median(values, count,    sorted, i) {
  for (i = 1; i <= count; i++) {
    sorted[i] = values[i]
  }
  sort_numeric(sorted, count)
  if (count % 2 == 1) {
    return sorted[(count + 1) / 2]
  }
  return (sorted[count / 2] + sorted[count / 2 + 1]) / 2
}

BEGIN {
  FS = OFS = "\t"
}

NR == 1 {
  for (i = 1; i <= NF; i++) {
    column[$i] = i
  }
  required[1] = "case"
  required[2] = "encode_mpps"
  required[3] = "decode_mpps"
  required[4] = "encoded_bytes"
  for (i = 1; i <= 4; i++) {
    if (!(required[i] in column)) {
      print "missing column: " required[i] > "/dev/stderr"
      exit 1
    }
  }
  next
}

{
  case_id = $column["case"]
  if (!(case_id in seen)) {
    seen[case_id] = 1
    order[++case_count] = case_id
  }
  count[case_id]++
  encode[case_id, count[case_id]] = $column["encode_mpps"]
  decode[case_id, count[case_id]] = $column["decode_mpps"]
  bytes[case_id, count[case_id]] = $column["encoded_bytes"]
}

END {
  if (case_count == 0) {
    exit 1
  }
  print "case", "trials", "median_encode_mpps", "median_decode_mpps", "encoded_bytes"
  for (c = 1; c <= case_count; c++) {
    case_id = order[c]
    delete enc_values
    delete dec_values
    delete byte_values
    for (i = 1; i <= count[case_id]; i++) {
      enc_values[i] = encode[case_id, i]
      dec_values[i] = decode[case_id, i]
      byte_values[i] = bytes[case_id, i]
    }
    enc_median = median(enc_values, count[case_id])
    dec_median = median(dec_values, count[case_id])
    byte_median = median(byte_values, count[case_id])
    log_encode += log(enc_median)
    log_decode += log(dec_median)
    printf "%s\t%d\t%.3f\t%.3f\t%.0f\n", case_id, count[case_id], enc_median, dec_median, byte_median
  }
  printf "geomean\t%d\t%.3f\t%.3f\t-\n", case_count, exp(log_encode / case_count), exp(log_decode / case_count)
}

