/** Save the recovery kit text as a local .txt download (setup and settings). */
export function downloadRecoveryKit(fileContent: string): void {
  const blob = new Blob([fileContent], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "VaultX-Recovery-Kit.txt";
  a.click();
  URL.revokeObjectURL(url);
}
