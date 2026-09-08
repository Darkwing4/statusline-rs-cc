(ns statusline-builder.boot
  (:require [statusline-builder.actions :as actions]
            [statusline-builder.catalogue :as catalogue]
            [statusline-builder.dom :as dom :refer [$ el]]
            [statusline-builder.drag-drop :as drag-drop]
            [statusline-builder.editor-state :as es]
            [statusline-builder.keyboard :as keyboard]
            [statusline-builder.render :as render]
            [statusline-builder.scenarios :as scenarios]
            [statusline-builder.sheets :as sheets]))

(def ^:private catalogue-url "segment-catalog.json")

(defn- scenario-options! []
  (let [select ($ "scenarioSelect")]
    (.replaceChildren select)
    (doseq [{:keys [id label]} scenarios/all]
      (let [option (el "option" "" label)]
        (set! (.-value option) id)
        (.append select option)))
    (set! (.-value select) (:scenario-id @es/state))))

(defn- reset-line! []
  (swap! es/state es/reset)
  (let [state @es/state]
    (set! (.-value ($ "scenarioSelect")) (:scenario-id state))
    (set! (.-value ($ "terminalWidth")) (str (:terminal-width state)))
    (render/inspector! state)
    (dom/announce! "Line reset to defaults.")))

(defn- on! [id event handler]
  (.addEventListener ($ id) event handler))

(defn- bind-events! []
  (on! "scenarioSelect" "change" #(actions/set-scenario! (.-value ($ "scenarioSelect"))))
  (on! "terminalWidth" "input" #(actions/set-terminal-width! (js/Number (.-value ($ "terminalWidth")))))
  (on! "resetButton" "click" reset-line!)
  (on! "ronButton" "click" sheets/open-ron!)
  (on! "copyRonButton" "click" sheets/copy-ron!)
  (on! "downloadButton" "click" sheets/download!)
  (on! "installButton" "click" sheets/open-install!)
  (on! "copyCommandButton" "click" sheets/copy-command!)
  (on! "removeSelectedButton" "click" #(when-let [selected (es/selected-segment @es/state)] (actions/remove! (:id selected))))
  (on! "lineCanvas" "click" #(when-not (.closest (.-target %) ".piece, .sep") (actions/select! es/line-selection)))
  (drag-drop/bind!)
  (keyboard/bind!))

(defn- watch-state! []
  (add-watch es/state ::render
             (fn [_ _ old new]
               (when (not= (es/config-key old) (es/config-key new))
                 (sheets/invalidate-session!))
               (render/changes! old new))))

(defn- start! [catalogue]
  (swap! es/state #(es/reset (es/install-catalogue % catalogue)))
  (dom/set-text! "catalogVersion" (str "v" (:catalogue-version @es/state)))
  (render/all! @es/state)
  (watch-state!))

(defn- show-catalogue-error! [error]
  (js/console.error "Could not load the segment catalogue." error)
  (let [message (or (ex-message error) (when (instance? js/Error error) (.-message error)) (str error))]
    (.replaceChildren ($ "shelf")
                      (el "p" "shelf-empty" (str "The segment catalogue could not be loaded (" message "). Generate it with: cargo run -- --schema > site/segment-catalog.json"))))
  (doseq [id ["installButton" "ronButton" "resetButton"]]
    (set! (.-disabled ($ id)) true)))

(defn- load-catalogue []
  (-> (js/fetch catalogue-url #js {:cache "no-cache"})
      (.then (fn [response]
               (if (.-ok response)
                 (.json response)
                 (throw (js/Error. (str catalogue-url ": HTTP " (.-status response)))))))
      (.then #(catalogue/validate (js->clj % :keywordize-keys true)))))

(defn init []
  (scenario-options!)
  (bind-events!)
  (set! (.-value ($ "terminalWidth")) (str (:terminal-width @es/state)))
  (.then (load-catalogue) start! show-catalogue-error!))
