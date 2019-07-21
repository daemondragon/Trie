import os
import subprocess
import re
import time



SEARCH_STRUCTURE_LIST = [
{
    "name": "art",
    "compiler": "target/release/TextMiningCompiler",
    "searcher": "target/release/TextMiningApp",
    "prefix": "ref"
},
{
    "name": "ref",
    "compiler": "ref/linux64/TextMiningCompiler",
    "searcher": "ref/linux64/TextMiningApp",
    "prefix": "ref"
}
]

FILES = [ {
    "name": "split/words_{}.txt".format(amount),
    "suffix": "_{}.bin".format(amount),
    "queries": [ "split/query_{}_{}_{}.txt".format(amount, proportion, 100 - proportion) for proportion in [25, 50, 75] ]
} for amount in [1000, 10000, 100000] ]

FILES.append({
    "name": "split/all.txt",
    "suffix": "_all.bin",
    "queries": [ "split/query_100000_{}_{}.txt".format(proportion, 100 - proportion) for proportion in [25, 50, 75] ]
})

DISTANCE_LIST = [0, 1, 2]


for search in SEARCH_STRUCTURE_LIST:
    for file in FILES:
        filename = "compiled/{}{}".format(search['prefix'], file['suffix'])
        if not os.path.exists(filename):
            print("Compiling {}...".format(filename))
            subprocess.run([search['compiler'], file['name'], filename], capture_output=True)

print("Compiled search structure creation done...")

for file in FILES:
    print("From compiled file \"{}\"".format(file['name']));

    for queries_filename in file['queries']:
        print("Testing query file \"{}\"".format(queries_filename));
        with open(queries_filename, "r") as queries_file:
            content = queries_file.read()

        # Add approx before
        content = re.sub(r"^", "approx ", content, flags=re.MULTILINE)
        content = re.sub(r"[0-9]+$", "", content, flags=re.MULTILINE)

        for distance in DISTANCE_LIST:
            # Change the distance
            to_search = re.sub(r"approx ", "approx {} ".format(distance), content, flags=re.MULTILINE)

            for search in SEARCH_STRUCTURE_LIST:
                times = []
                args = [search['searcher'], "compiled/{}{}".format(search['prefix'], file['suffix'])]

                for i in range(10):
                    start = time.monotonic()
                    subprocess.run(args, input=to_search, encoding="ascii", stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    end = time.monotonic()

                    times.append(end - start)

                times = sorted(times)[2:-2]
                min_time, median, max_time = times[0], times[len(times) // 2], times[-1]
                error = max(median - min_time, max_time - median)

                query = len(content.splitlines()) / median

                print("struct: {}, distance: {}, time: {} ms (+- {} ms) => {} query/sec (+- {} query)".format(
                    search['name'],
                    distance,
                    int(median * 1000),
                    int(error * 1000),
                    int(query),
                    int(error * query / (median * 1000))
                ));