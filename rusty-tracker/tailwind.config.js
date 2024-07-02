module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  plugins: [
    require("@catppuccin/tailwindcss")({
     // which flavour of colours to use by default, in the `:root`
      defaultFlavour: "macchiato",
    }),
  ],
};
