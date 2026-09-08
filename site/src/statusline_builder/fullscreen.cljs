(ns statusline-builder.fullscreen
  (:require [statusline-builder.dom :as dom :refer [$]]))

(defn- supported? []
  (some? (.-requestFullscreen ($ "lineFrame"))))

(defn active? []
  (some? (.-fullscreenElement js/document)))

(defn toggle! []
  (when (supported?)
    (-> (if (active?)
          (.exitFullscreen js/document)
          (.requestFullscreen ($ "lineFrame")))
        (.catch #(js/console.error "Could not switch full screen." %)))))

(defn- sync-button! []
  (let [button ($ "fullscreenButton")
        label (if (active?) "Exit full screen" "Full screen")]
    (.setAttribute button "aria-label" label)
    (set! (.-title button) (str label " (f)"))))

(defn bind! []
  (dom/set-hidden! "fullscreenButton" (not (supported?)))
  (.addEventListener ($ "fullscreenButton") "click" toggle!)
  (.addEventListener js/document "fullscreenchange" sync-button!))
