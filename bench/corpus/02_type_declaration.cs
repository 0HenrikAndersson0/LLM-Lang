public class Order
{
    public List<LineItem> LineItems { get; set; }
    public Customer Customer { get; set; }
    public decimal Total()
        => LineItems.Select(li => li.Amount).Sum();
}