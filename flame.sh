binary="target/release/TextMiningApp"
prefix="art"
#binary="ref/linux64/TextMiningApp"
#prefix="ref"
ratio_list=(25 50 75)
amount_list=(1000 10000 100000)

read -p "Select the wanted distance: " distance

mkdir -p "record"

for amount in "${amount_list[@]}"
do
    compiled_file="compiled/${prefix}_${amount}.bin"

    for ratio in "${ratio_list[@]}"
    do
        query_file="split/query_${amount}_$(($ratio))_$((100 - $ratio)).txt"
        record_file="record/record_${prefix}_${amount}_${ratio}_${distance}.perf"

        if [ ! -f "$record_file" ]; then
            cat $query_file | sed -E "s/^/approx $distance /" | sed -E "s/\s[0-9]+$//" | sudo perf record -o $record_file -g $binary $compiled_file
        else
            echo "file $query_file already exist, skipping it..."
        fi
    done
done

record_file_list=$(find record -type f -name "*.perf" | sort)

echo "$record_file_list" | cat -n

read -p "Select the wanted file: " record_line

sudo perf report -i $(echo -n "$record_file_list" | sed -n "${record_line}p")
