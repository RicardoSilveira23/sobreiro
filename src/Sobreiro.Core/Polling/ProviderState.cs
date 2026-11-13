namespace Sobreiro.Core.Polling;

/// <summary>Where a provider's poll stands: Off and Ok, or the reason its numbers stopped.</summary>
public enum ProviderState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Offline, BadAnswer }
