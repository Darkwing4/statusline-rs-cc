(ns statusline-builder.sheets
  (:require [clojure.string :as str]
            [statusline-builder.dom :as dom :refer [$ el]]
            [statusline-builder.editor-state :as es]
            [statusline-builder.install-command :as install-command]
            [statusline-builder.ron :as ron]))

(defonce ^:private last-download-url (atom ""))
(defonce ^:private copied-timers (atom {}))

(def ^:private copied-label-ms 2000)

(defn invalidate-session! []
  (dom/set-hidden! "sessionResult" true)
  (dom/set-hidden! "sessionError" true)
  (dom/set-text! "sessionError" "")
  (set! (.-value ($ "commandOutput")) ""))

(defn- show-copied! [button-id message]
  (let [button ($ button-id)]
    (js/clearTimeout (get @copied-timers button-id))
    (.add (.-classList button) "is-copied")
    (set! (.-textContent button) "Copied to clipboard")
    (dom/announce! message)
    (swap! copied-timers assoc button-id
           (js/setTimeout (fn []
                            (.remove (.-classList button) "is-copied")
                            (set! (.-textContent button) "Copy"))
                          copied-label-ms))))

(defn- copy-with-selection! [select!]
  (js/Promise. (fn [resolve reject]
                 (select!)

                 (if (.execCommand js/document "copy")
                   (resolve)
                   (reject (js/Error. "The browser refused to copy."))))))

(defn- copy-text! [text select!]
  (if (and (.-clipboard js/navigator) js/globalThis.isSecureContext)
    (.catch (.writeText (.-clipboard js/navigator) text)
            (fn [error]
              (js/console.error "Clipboard API failed." error)
              (copy-with-selection! select!)))
    (copy-with-selection! select!)))

(defn- select-ron! []
  (.selectAllChildren (js/getSelection) ($ "ronOutput")))

(defn- select-command! []
  (let [output ($ "commandOutput")]
    (.focus output)
    (.select output)))

(defn copy-ron! []
  (-> (copy-text! (.-textContent ($ "ronOutput")) select-ron!)
      (.then #(show-copied! "copyRonButton" "config.ron copied."))
      (.catch #(js/console.error "Could not copy config.ron." %))))

(defn copy-command! []
  (let [command (.-value ($ "commandOutput"))]

    (when (seq command)
      (-> (copy-text! command select-command!)
          (.then #(show-copied! "copyCommandButton" "Install command copied."))
          (.catch #(js/console.error "Could not copy the install command." %))))))

(defn open-ron! []
  (let [state @es/state
        problems (ron/config-problems state)]
    (dom/set-hidden! "ronProblems" (empty? problems))
    (dom/set-text! "ronProblems" (str/join " " problems))
    (dom/set-text! "ronOutput" (ron/generate state))
    (.showModal ($ "ronDialog"))
    (copy-ron!)))

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
        (.then copy-command!)
        (.catch (fn [error]
                  (js/console.error "Could not build the install command." error)
                  (show-error! (if (instance? js/Error error) (.-message error) "Could not build the install command."))))
        (.finally #(set! (.-disabled copy-button) false)))))

(defn open-install! []
  (.showModal ($ "installDialog"))
  (build-command!))
