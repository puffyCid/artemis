# Install some node modules needed for Apollo
@_node_modules:
  echo "Installing DaisyUI and TailWindCSS Typography"
  cd ../.. && npm i -D daisyui@latest
  cd ../.. && npm install -D @tailwindcss/typography