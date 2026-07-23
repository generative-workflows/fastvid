#!/usr/bin/gawk -f
BEGIN {
    FS = OFS = "\t"
}

NR == 1 {
    for (column = 1; column <= NF; column++) {
        index_of[$column] = column
    }
    next
}

{
    quality = $(index_of["quality"])
    threads = $(index_of["threads"])
    gop = $(index_of["gop"])
    input = $(index_of["input"])
    target = $(index_of["target_frame"])
    key = quality SUBSEP threads SUBSEP gop SUBSEP input SUBSEP target
    trial_count[key]++
    latency[key, trial_count[key]] = $(index_of["access_ms"])
    amplification[key] = $(index_of["access_amplification"])
}

END {
    for (key in trial_count) {
        delete ordered
        for (trial = 1; trial <= trial_count[key]; trial++) {
            ordered[trial] = latency[key, trial]
        }
        asort(ordered)
        count = trial_count[key]
        if (count % 2 == 0) {
            target_median = (ordered[count / 2] + ordered[count / 2 + 1]) / 2
        } else {
            target_median = ordered[(count + 1) / 2]
        }
        split(key, fields, SUBSEP)
        group = fields[1] SUBSEP fields[2] SUBSEP fields[3]
        group_count[group]++
        group_latency[group, group_count[group]] = target_median
        group_amplification[group] += amplification[key]
        if (!(group in worst_latency) || target_median > worst_latency[group]) {
            worst_latency[group] = target_median
            worst_input[group] = fields[4]
            worst_target[group] = fields[5]
        }
    }

    print "quality", "threads", "gop", "targets", "median_access_ms", \
        "p95_access_ms", "worst_access_ms", "mean_amplification", \
        "worst_input", "worst_target"
    for (group in group_count) {
        delete ordered
        for (target_index = 1; target_index <= group_count[group]; target_index++) {
            ordered[target_index] = group_latency[group, target_index]
        }
        asort(ordered)
        count = group_count[group]
        if (count % 2 == 0) {
            median = (ordered[count / 2] + ordered[count / 2 + 1]) / 2
        } else {
            median = ordered[(count + 1) / 2]
        }
        p95_index = int((count * 95 + 99) / 100)
        split(group, fields, SUBSEP)
        print fields[1], fields[2], fields[3], count, sprintf("%.3f", median), \
            sprintf("%.3f", ordered[p95_index]), \
            sprintf("%.3f", worst_latency[group]), \
            sprintf("%.3f", group_amplification[group] / count), \
            worst_input[group], worst_target[group]
    }
}
