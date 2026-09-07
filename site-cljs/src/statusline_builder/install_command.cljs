(ns statusline-builder.install-command
  (:require [clojure.string :as str]))

(def ^:private installer-url "https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh")

(defn supported? []
  (exists? js/CompressionStream))

(defn- base64-url [bytes]
  (-> (js/btoa (apply str (map js/String.fromCharCode bytes)))
      (str/replace "+" "-")
      (str/replace "/" "_")
      (str/replace "=" "")))

(defn encode [ron]
  (-> (js/Blob. #js [ron])
      (.stream)
      (.pipeThrough (js/CompressionStream. "gzip"))
      (js/Response.)
      (.arrayBuffer)
      (.then #(base64-url (js/Array.from (js/Uint8Array. %))))))

(defn command [code]
  (str "curl -fsSL " installer-url " | STATUSLINE_INSTALL_CONFIG=" code " sh"))

(defn describe-size [ron code]
  (str (.-length (.encode (js/TextEncoder.) ron)) " B RON · " (count code) " char code"))
