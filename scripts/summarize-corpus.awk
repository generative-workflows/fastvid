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
  delete sorted
  for (i = 1; i <= count; i++) {
    sorted[i] = values[i]
  }
  sort_numeric(sorted, count)
  if (count % 2) {
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
  next
}

{
  key = $column["input"] SUBSEP $column["quality"] SUBSEP \
    $column["threads"] SUBSEP $column["gop"]
  if (!(key in seen)) {
    seen[key] = 1
    order[++group_count] = key
    input[key] = $column["input"]
    quality[key] = $column["quality"]
    threads[key] = $column["threads"]
    gop[key] = $column["gop"]
  }
  count[key]++
  encode[key, count[key]] = $column["encode_mpps"]
  decode[key, count[key]] = $column["decode_mpps"]
  bytes[key, count[key]] = $column["encoded_bytes"]
  psnr[key, count[key]] = $column["y_psnr"]
  ssim[key, count[key]] = $column["y_block_ssim"]
}

END {
  print "input", "quality", "threads", "gop", "trials", \
    "median_encode_mpps", "median_decode_mpps", "encoded_bytes", \
    "y_psnr", "y_block_ssim"
  for (group = 1; group <= group_count; group++) {
    key = order[group]
    delete values
    for (i = 1; i <= count[key]; i++) values[i] = encode[key, i]
    enc = median(values, count[key])
    delete values
    for (i = 1; i <= count[key]; i++) values[i] = decode[key, i]
    dec = median(values, count[key])
    delete values
    for (i = 1; i <= count[key]; i++) values[i] = bytes[key, i]
    encoded = median(values, count[key])
    delete values
    for (i = 1; i <= count[key]; i++) values[i] = psnr[key, i]
    y_psnr = median(values, count[key])
    delete values
    for (i = 1; i <= count[key]; i++) values[i] = ssim[key, i]
    y_ssim = median(values, count[key])
    printf "%s\t%d\t%d\t%d\t%d\t%.3f\t%.3f\t%.0f\t%.6f\t%.8f\n", \
      input[key], quality[key], threads[key], gop[key], count[key], \
      enc, dec, encoded, y_psnr, y_ssim
  }
}

