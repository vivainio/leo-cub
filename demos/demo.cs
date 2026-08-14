using System;

namespace Cub.Demos
{
    public interface IGreeter
    {
        string Greet(string name);
    }

    public record Greeting(string Message);

    public class Greeter : IGreeter
    {
        public Greeter(string prefix)
        {
            Prefix = prefix;
        }

        public string Prefix { get; }

        public string Greet(string name)
        {
            return $"{Prefix}, {name}!";
        }
    }
}
