namespace Humanizer;

class LithuanianNumberFormDetector
{
    public static LithuanianNumberForm Detect(long number)
    {
        var tens = number%100/10;
        var units = number%10;
        if (tens==1||units==0)
        {
            return LithuanianNumberForm.GenitivePlural;
        }
        if (units==1)
        {
            return LithuanianNumberForm.Singular;
        }
        return LithuanianNumberForm.Plural;
    }
}
