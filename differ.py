import os
import subprocess
import re

SEARCH_STRUCTURE_LIST = [
{
    "name": "art",
    "compiler": "target/release/TextMiningCompiler",
    "searcher": "target/release/TextMiningApp",
    "prefix": "art"
},
{
    "name": "ref",
    "compiler": "ref/linux64/TextMiningCompiler",
    "searcher": "ref/linux64/TextMiningApp",
    "prefix": "ref"
}
]

AMOUNT_LIST = [1000, 10000, 100000]
DISTANCE_LIST = [0, 1, 2]
PROPORTION_LIST = [25, 50, 75]

FILES = [ {
    "name": "split/words_{}.txt".format(amount),
    "suffix": "_{}.bin".format(amount),
    "queries": [ "split/query_{}_{}_{}.txt".format(amount, proportion, 100 - proportion) for proportion in [25, 50, 75] ]
} for amount in [1000, 10000, 100000] ]


FILES.append({
    "name": "all.txt",
    "suffix": "_all.bin",
    "queries": [ "all.txt" ]
})

for search in SEARCH_STRUCTURE_LIST:
    for file in FILES:
        filename = "compiled/{}{}".format(search['prefix'], file['suffix'])
        if not os.path.exists(filename):
            print("Compiling {}...".format(filename))
            subprocess.run([search['compiler'], file['name'], filename], capture_output=True)

print("Compiled search structure creation done...")
for amount in AMOUNT_LIST:
   for file in FILES:
    print("From compiled file \"{}\"".format(file['name']));

    for queries_filename in file['queries']:
        print("Testing query file \"{}\"".format(queries_filename));
        with open(queries_filename, "r") as queries_file:
            content = queries_file.read()

        # Add approx before
        content = re.sub(r"^", "approx ", content, flags=re.MULTILINE)
        content = re.sub(r"\s+[0-9]+$", "", content, flags=re.MULTILINE)

        for distance in DISTANCE_LIST:
            # Change the distance
            to_search = re.sub(r"approx ", "approx {} ".format(distance), content, flags=re.MULTILINE)

            results = []

            for search in SEARCH_STRUCTURE_LIST:
                print("Launching {}".format(search['name']))
                args = [search['searcher'], "compiled/{}{}".format(search['prefix'], file['suffix'])]
                process = subprocess.run(args, input=to_search, encoding="ascii", stdout=subprocess.PIPE)

                results.append({
                    "stdout" : process.stdout
                })

            for index in range(len(results) - 1):
                first_name = SEARCH_STRUCTURE_LIST[index]['name']
                second_name = SEARCH_STRUCTURE_LIST[index + 1]['name']

                if results[index]['stdout'] != results[index + 1]['stdout']:
                    print("Error while diffing stdout of {} and {}".format(first_name, second_name))
                    first_output = results[index]['stdout'].splitlines()
                    second_output = results[index + 1]['stdout'].splitlines()

                    if len(first_output) != len(second_output):
                        print("Output {} and {} have different len ({} and {})".format(first_name, second_name, len(first_output), len(second_output)))
                    else:
                        for i in range(len(first_output)):
                            if first_output[i] != second_output[i]:
                                print("Diff while searching for {}:".format(to_search.splitlines()[i]), first_output[i], second_output[i], sep="\n")
                else:
                    print("{} and {} have the same output".format(first_name, second_name))