static int SumAll(List<int> xs)
{
    var acc = 0;
    foreach (var x in xs)
        acc += x;
    return acc;
}