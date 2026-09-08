(ns statusline-builder.sheets
  (:require [clojure.string :as str]
            [statusline-builder.dom :as dom :refer [$ el]]
            [statusline-builder.editor-state :as es]
            [statusline-builder.install-command :as install-command]
            [statusline-builder.ron :as ron]))

(defonce ^:private last-download-url (atom ""))

(defn invalidate-session! []
  (dom/set-hidden! "sessionResult" true)
  (dom/set-hidden! "sessionError" true)
  (dom/set-text! "sessionError" "")
  (set! (.-value ($ "commandOutput")) ""))

(defn open-ron! []
  (let [state @es/state
        problems (ron/config-problems state)]
    (dom/set-hidden! "ronProblems" (empty? problems))
    (dom/set-text! "ronProblems" (str/join " " problems))
    (dom/set-text! "ronOutput" (ron/generate state))
    (.showModal ($ "ronDialog"))))

(defn download! []
  (when (seq @last-download-url)
    (js/URL.revokeObjectURL @last-download-url))
  (let [url (js/URL.createObjectURL (js/Blob. #js [(ron/generate @es/state)] #js {:type "text/plain;charset=utf-8"}))
        link (el "a")]
    (reset! last-download-url url)
    (set! (.-href link) url)
    (set! (.-download link) "config.ron")
    (.append (.-body js/document) link)
    (.click link)
    (.remove link)
    (dom/announce! "config.ron downloaded.")))

(defn- show-error! [message]
  (dom/set-text! "sessionError" message)
  (dom/set-hidden! "sessionError" false)
  (dom/announce! "Install command could not be created."))

(defn- show-command! [ron code]
  (dom/set-text! "configSizeOutput" (install-command/describe-size ron code))
  (set! (.-value ($ "commandOutput")) (install-command/command code))
  (dom/set-hidden! "sessionResult" false)
  (dom/announce! "Install command created."))

(defn- build-command! []
  (let [state @es/state
        ron (ron/generate state)
        problems (ron/config-problems state)
        copy-button ($ "copyCommandButton")]
    (set! (.-disabled copy-button) true)
    (dom/set-hidden! "sessionResult" true)
    (dom/set-hidden! "sessionError" true)
    (-> (cond
          (seq problems) (js/Promise.reject (js/Error. (str/join " " problems)))
          (not (install-command/supported?)) (js/Promise.reject (js/Error. "This browser cannot compress the config. Download the RON instead."))
          :else (install-command/encode ron))
        (.then (fn [code]
                 (if (= ron (ron/generate @es/state))
                   (show-command! ron code)
                   (throw (js/Error. "The configuration changed while the command was being built. Build it again.")))))
        (.catch (fn [error]
                  (js/console.error "Could not build the install command." error)
                  (show-error! (if (instance? js/Error error) (.-message error) "Could not build the install command."))))
        (.finally #(set! (.-disabled copy-button) false)))))

(defn open-install! []
  (.showModal ($ "installDialog"))
  (build-command!))

(defn- copied! []
  (let [button ($ "copyCommandButton")]
    (set! (.-textContent button) "Copied")
    (dom/announce! "Install command copied.")
    (.addEventListener button "blur" #(set! (.-textContent button) "Copy") #js {:once true})))

(defn- copy-fallback! []
  (let [output ($ "commandOutput")]
    (.focus output)
    (.select output)
    (.execCommand js/document "copy")
    (copied!)))

(defn copy-command! []
  (let [command (.-value ($ "commandOutput"))]
    (when (seq command)
      (if (and (.-clipboard js/navigator) js/globalThis.isSecureContext)
        (-> (.writeText (.-clipboard js/navigator) command)
            (.then copied!)
            (.catch (fn [error]
                      (js/console.error "Clipboard API failed." error)
                      (copy-fallback!))))
        (copy-fallback!)))))
