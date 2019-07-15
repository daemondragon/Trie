ratio_list=(25 50 75)
amount_list=(1000 10000 100000)
test_amount_max=3000

mkdir -p split

for amount in "${amount_list[@]}"
do
    head -n $amount < all.txt > "split/words_${amount}.txt"

    #test_amount_max=$amount #To allows testing for art, who is too fast

    for ratio in "${ratio_list[@]}"
    do
        test_amount=$(echo -e "$test_amount_max\n$amount" | sort -g | head -n 1)
        included=$(($ratio * $test_amount / 100))
        excluded=$(((100 - $ratio) * $test_amount / 100))

        head -n $included < all.txt > "split/query_${amount}_$(($ratio))_$((100 - $ratio)).txt"
        tail -n $excluded < all.txt >> "split/query_${amount}_$(($ratio))_$((100 - $ratio)).txt"

    done
done
