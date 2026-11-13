using Sobreiro.Core.Baseline;

namespace Sobreiro.Tests;

public class BaselineMathTests
{
    [Fact]
    public void Cpu_per_hour_scales_the_window_to_an_hour() =>
        Assert.Equal(6.0, BaselineMath.CpuSecondsPerHour(
            TimeSpan.FromSeconds(2.0), TimeSpan.FromSeconds(2.9), TimeSpan.FromMinutes(9)), 6);

    [Fact]
    public void Cpu_per_hour_of_an_idle_process_is_zero() =>
        Assert.Equal(0, BaselineMath.CpuSecondsPerHour(
            TimeSpan.FromSeconds(2), TimeSpan.FromSeconds(2), TimeSpan.FromMinutes(9)));

    [Fact]
    public void Cpu_per_hour_needs_a_window() =>
        Assert.Throws<ArgumentOutOfRangeException>(() => BaselineMath.CpuSecondsPerHour(
            TimeSpan.Zero, TimeSpan.FromSeconds(1), TimeSpan.Zero));

    [Fact]
    public void Median_of_an_odd_count_is_the_middle_one() =>
        Assert.Equal(TimeSpan.FromMilliseconds(30), BaselineMath.Median(
            [TimeSpan.FromMilliseconds(900), TimeSpan.FromMilliseconds(20), TimeSpan.FromMilliseconds(30)]));

    [Fact]
    public void Median_of_an_even_count_is_between_the_middle_two() =>
        Assert.Equal(TimeSpan.FromMilliseconds(25), BaselineMath.Median(
            [TimeSpan.FromMilliseconds(40), TimeSpan.FromMilliseconds(10), TimeSpan.FromMilliseconds(20), TimeSpan.FromMilliseconds(30)]));

    [Fact]
    public void Median_of_nothing_is_nothing() => Assert.Null(BaselineMath.Median([]));
}
