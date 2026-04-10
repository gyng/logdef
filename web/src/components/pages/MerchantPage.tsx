import { useCallback, useState } from "react";
import { getBridge } from "../../bridge";
import type { MerchantSnapshot } from "../../bridge/types";
import { t } from "../../i18n";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
}

function readMerchant(): MerchantSnapshot {
  return JSON.parse(getBridge().get_merchant_state()) as MerchantSnapshot;
}

export function MerchantPage({ sendCommand, refreshPhase }: Props) {
  const [snapshot, setSnapshot] = useState<MerchantSnapshot>(readMerchant);

  const refresh = useCallback(() => {
    setSnapshot(readMerchant());
  }, []);

  return (
    <div className="merchant-page">
      <h2>{t("merchant.title")}</h2>
      <p className="gold">{t("merchant.gold", { gold: snapshot.gold })}</p>

      <div className="merchant-stock">
        {snapshot.items.map((item) => (
          <div key={item.id} className="merchant-item">
            <h3>{item.name}</h3>
            <p>{item.description}</p>
            <p className="price">{t("merchant.price", { gold: item.price })}</p>
            <button
              disabled={snapshot.gold < item.price}
              onClick={() => {
                sendCommand({ BuyItem: { item_index: item.index } });
                refresh();
              }}
            >
              {t("merchant.buy")}
            </button>
          </div>
        ))}
      </div>

      <button
        className="continue-button"
        onClick={() => {
          sendCommand("ContinueJourney");
          refreshPhase();
        }}
      >
        {t("merchant.leave")}
      </button>
    </div>
  );
}
