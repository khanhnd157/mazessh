import { memo } from "react";
import { GitFork, Globe, Waypoints } from "lucide-react";
import type { Provider } from "@/types";

interface ProviderIconProps {
  provider: Provider;
  size?: number;
  className?: string;
}

export const ProviderIcon = memo(function ProviderIcon({ provider, size = 16, className = "" }: ProviderIconProps) {
  const providerKey = typeof provider === "string" ? provider : "custom";

  switch (providerKey) {
    case "github":
      return <GitFork size={size} className={`text-gray-400 ${className}`} />;
    case "gitlab":
      return <GitFork size={size} className={`text-orange-400 ${className}`} />;
    case "gitea":
      return <Waypoints size={size} className={`text-green-400 ${className}`} />;
    case "bitbucket":
      return <Globe size={size} className={`text-blue-400 ${className}`} />;
    default:
      return <Globe size={size} className={`text-purple-400 ${className}`} />;
  }
});
