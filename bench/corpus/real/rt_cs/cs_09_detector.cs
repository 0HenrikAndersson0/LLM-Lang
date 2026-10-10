namespace Humanizer;

class RussianGrammaticalNumberDetector
{
    internal RussianGrammaticalNumber Detect(long number)
    {
        var tens = number%100/10;
        if (tens!=1)
        {
            var unity = number%10;
            if (unity==1)
            {
                return RussianGrammaticalNumber.Singular;
            }
            if (unity is >1 and <5)
            {
                return RussianGrammaticalNumber.Paucal;
            }
        }
        return RussianGrammaticalNumber.Plural;
    }
}
