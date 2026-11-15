using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class BridgeLocatorTests
{
    private const string App = @"C:\Apps\Sobreiro\app";

    [Fact]
    public void Finds_the_bridge_beside_the_app()
    {
        var beside = Path.GetFullPath(Path.Combine(App, "sobreiro-bridge.exe"));
        Assert.Equal(beside, BridgeLocator.Find(App, p => p == beside));
    }

    [Fact]
    public void Finds_the_bridge_in_the_sibling_bridge_folder()
    {
        var sibling = @"C:\Apps\Sobreiro\bridge\sobreiro-bridge.exe";
        Assert.Equal(sibling, BridgeLocator.Find(App, p => p == sibling));
    }

    [Fact]
    public void Prefers_the_bridge_beside_the_app()
    {
        var beside = Path.GetFullPath(Path.Combine(App, "sobreiro-bridge.exe"));
        Assert.Equal(beside, BridgeLocator.Find(App, _ => true));
    }

    [Fact]
    public void Returns_null_when_neither_exists()
    {
        Assert.Null(BridgeLocator.Find(App, _ => false));
    }

    [Fact]
    public void Own_is_the_installed_copy_when_the_exe_carries_the_bridge() =>
        Assert.Equal(@"C:\Data\sobreiro-bridge.exe", BridgeLocator.Own(true, @"C:\Data\sobreiro-bridge.exe", App, _ => true));

    [Fact]
    public void Own_is_the_bridge_beside_the_app_otherwise()
    {
        var beside = Path.GetFullPath(Path.Combine(App, "sobreiro-bridge.exe"));
        Assert.Equal(beside, BridgeLocator.Own(false, @"C:\Data\sobreiro-bridge.exe", App, p => p == beside));
    }

    [Fact]
    public void Own_is_null_when_not_carried_and_none_beside() =>
        Assert.Null(BridgeLocator.Own(false, @"C:\Data\sobreiro-bridge.exe", App, _ => false));
}
